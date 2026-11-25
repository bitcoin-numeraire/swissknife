use std::sync::{
    atomic::{AtomicU64, Ordering},
    Arc,
};

use anyhow::{ensure, Context};
use async_trait::async_trait;
use chrono::DateTime;
use serde::Deserialize;
use serde_json::json;

use super::{cln_rest_client::ClnRestRpc, ClnRestClientConfig};
use crate::{
    application::{composition::AppServices, errors::LightningError},
    domains::event::LnInvoicePaidEvent,
    infra::lightning::EventsListener,
};

const PAGE_SIZE: usize = 100;

#[derive(Clone, Copy)]
enum OffchainIndex {
    Invoices,
    Sendpays,
}

impl OffchainIndex {
    fn subsystem(self) -> &'static str {
        match self {
            Self::Invoices => "invoices",
            Self::Sendpays => "sendpays",
        }
    }

    fn list_method(self) -> &'static str {
        match self {
            Self::Invoices => "listinvoices",
            Self::Sendpays => "listsendpays",
        }
    }
}

#[derive(Deserialize)]
struct UpdatePage {
    #[serde(rename = "invoices", alias = "payments")]
    entries: Vec<Update>,
}

#[derive(Deserialize)]
struct Update {
    updated_index: u64,
    payment_hash: String,
    status: String,
    amount_received_msat: Option<u64>,
    paid_at: Option<i64>,
}

impl Update {
    fn into_paid_invoice(self) -> anyhow::Result<LnInvoicePaidEvent> {
        Ok(LnInvoicePaidEvent {
            payment_hash: self.payment_hash,
            amount_received_msat: self
                .amount_received_msat
                .context("CLN paid invoice is missing amount_received_msat")?,
            fee_msat: 0,
            payment_time: self
                .paid_at
                .and_then(|t| DateTime::from_timestamp(t, 0))
                .context("CLN paid invoice has no valid settlement time")?,
        })
    }
}

pub struct ClnRestListener {
    rpc: ClnRestRpc,
    services: Arc<AppServices>,
    next_invoice: AtomicU64,
    next_sendpay: AtomicU64,
}

impl ClnRestListener {
    pub async fn new(config: ClnRestClientConfig, services: Arc<AppServices>) -> Result<Self, LightningError> {
        let rpc = ClnRestRpc::new(&config).await?;
        // Capture both boundaries before startup replay. A reconnect keeps these cursors
        // so that a wake-up or a failed database write never skips a settlement.
        let (invoices, sendpays) =
            tokio::try_join!(rpc.wait("invoices", "updated", 0), rpc.wait("sendpays", "updated", 0))
                .map_err(|e| LightningError::Listener(e.to_string()))?;
        Ok(Self {
            rpc,
            services,
            next_invoice: AtomicU64::new(invoices.saturating_add(1)),
            next_sendpay: AtomicU64::new(sendpays.saturating_add(1)),
        })
    }

    async fn follow_offchain(&self, index: OffchainIndex, cursor: &AtomicU64) -> anyhow::Result<()> {
        loop {
            let mut next = cursor.load(Ordering::Relaxed);
            let latest = self.rpc.wait(index.subsystem(), "updated", next).await?;
            let page: UpdatePage = self
                .rpc
                .post(
                    index.list_method(),
                    &json!({
                        "index": "updated", "start": next, "limit": PAGE_SIZE
                    }),
                )
                .await?;
            let last_page = page.entries.len() < PAGE_SIZE;
            for update in page.entries {
                ensure!(
                    update.updated_index >= next,
                    "CLN returned unordered {} updates",
                    index.subsystem()
                );
                let processed = update.updated_index;
                match index {
                    OffchainIndex::Invoices if update.status == "paid" => {
                        self.services.event.invoice_paid(update.into_paid_invoice()?).await?;
                    }
                    OffchainIndex::Sendpays => {
                        // Attempt-level updates require a fresh aggregate payment status.
                        self.services.payment.sync_one(update.payment_hash).await?;
                    }
                    OffchainIndex::Invoices => {}
                }
                next = processed.saturating_add(1);
                cursor.store(next, Ordering::Relaxed);
            }
            // Deletions leave index gaps. A short page establishes that every retained
            // update through the observed high-water mark has been considered.
            if last_page {
                cursor.store(next.max(latest.saturating_add(1)), Ordering::Relaxed);
            }
        }
    }

    async fn follow_chainmoves(&self) -> anyhow::Result<()> {
        let mut latest = self.rpc.wait("chainmoves", "created", 0).await?;
        loop {
            // The Bitcoin service projects the whole batch and commits the durable
            // cursor only after the last wallet update succeeds.
            self.services.bitcoin.sync().await?;
            latest = self.rpc.wait("chainmoves", "created", latest.saturating_add(1)).await?;
        }
    }
}

#[async_trait]
impl EventsListener for ClnRestListener {
    async fn listen(&self) -> Result<(), LightningError> {
        tokio::try_join!(
            self.follow_offchain(OffchainIndex::Invoices, &self.next_invoice),
            self.follow_offchain(OffchainIndex::Sendpays, &self.next_sendpay),
            self.follow_chainmoves(),
        )
        .map_err(|e| LightningError::Listener(e.to_string()))?;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::application::composition::MockAppServicesBuilder;
    use std::{sync::atomic::AtomicUsize, time::Duration};
    use wiremock::{
        matchers::{body_partial_json, path},
        Mock, MockServer, ResponseTemplate,
    };

    #[tokio::test]
    async fn replays_full_pages_and_skips_deleted_indexes_only_after_a_short_page() {
        let server = MockServer::start().await;
        let config = ClnRestClientConfig {
            endpoint: server.uri(),
            rune: "test".into(),
            connect_timeout: Duration::from_secs(2),
            connection_verbose: false,
            timeout: Duration::from_secs(2),
            accept_invalid_certs: false,
            accept_invalid_hostnames: false,
            payment_timeout: Duration::from_secs(2),
            maxfee: None,
            ws_min_reconnect_delay: Duration::from_secs(1),
            ws_max_reconnect_delay: Duration::from_secs(2),
            ca_cert_path: None,
        };
        for start in [1, 101] {
            Mock::given(path("/v1/wait"))
                .and(body_partial_json(
                    json!({"subsystem":"invoices","indexname":"updated","nextvalue":start}),
                ))
                .respond_with(ResponseTemplate::new(200).set_body_json(json!({"updated":105})))
                .expect(1)
                .mount(&server)
                .await;
        }
        for (start, end) in [(1, 100), (101, 101)] {
            let rows: Vec<_> = (start..=end)
                .map(|n| {
                    json!({
                        "updated_index":n, "status":"paid", "payment_hash":format!("{n:064x}"),
                        "paid_at":1700000000, "amount_received_msat":n
                    })
                })
                .collect();
            Mock::given(path("/v1/listinvoices"))
                .and(body_partial_json(json!({"start":start,"limit":100,"index":"updated"})))
                .respond_with(ResponseTemplate::new(200).set_body_json(json!({"invoices":rows})))
                .expect(1)
                .mount(&server)
                .await;
        }
        Mock::given(path("/v1/wait"))
            .and(body_partial_json(json!({"nextvalue":106})))
            .respond_with(ResponseTemplate::new(503).set_body_string("node offline"))
            .expect(1)
            .mount(&server)
            .await;
        let mut services = MockAppServicesBuilder::new();
        let count = AtomicUsize::new(0);
        services.event.expect_invoice_paid().times(101).returning(move |event| {
            let expected = count.fetch_add(1, Ordering::Relaxed) + 1;
            assert_eq!(event.amount_received_msat, expected as u64);
            assert_eq!(event.payment_hash, format!("{expected:064x}"));
            Ok(())
        });
        let listener = ClnRestListener {
            rpc: ClnRestRpc::new(&config).await.unwrap(),
            services: Arc::new(services.build()),
            next_invoice: AtomicU64::new(1),
            next_sendpay: AtomicU64::new(1),
        };
        let error = listener
            .follow_offchain(OffchainIndex::Invoices, &listener.next_invoice)
            .await
            .unwrap_err();
        assert!(error.to_string().contains("node offline"));
        assert_eq!(listener.next_invoice.load(Ordering::Relaxed), 106);
    }
}
