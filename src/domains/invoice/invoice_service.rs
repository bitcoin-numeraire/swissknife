use async_trait::async_trait;
use chrono::Utc;
use tracing::{debug, info, trace};
use uuid::Uuid;

use std::sync::Arc;

use crate::{
    application::{
        composition::{AppStore, Ledger},
        errors::{ApplicationError, DataError},
    },
    domains::{
        asset::{Protocol, NATIVE_ASSET_REF},
        bitcoin::BtcNetwork,
        event::{EventUseCases, LnInvoicePaidEvent},
    },
    infra::lightning::LnClient,
};

use super::{Invoice, InvoiceFilter, InvoiceStatus, InvoiceUseCases};

const DEFAULT_INVOICE_DESCRIPTION: &str = "Numeraire Invoice";

pub struct InvoiceService {
    store: AppStore,
    ln_client: Arc<dyn LnClient>,
    invoice_expiry: u32,
    events: Arc<dyn EventUseCases>,
    network: BtcNetwork,
}

impl InvoiceService {
    pub fn new(
        store: AppStore,
        ln_client: Arc<dyn LnClient>,
        invoice_expiry: u32,
        events: Arc<dyn EventUseCases>,
        network: BtcNetwork,
    ) -> Self {
        InvoiceService {
            store,
            ln_client,
            invoice_expiry,
            events,
            network,
        }
    }

    async fn ensure_wallet_network(&self, wallet_id: Uuid) -> Result<(), ApplicationError> {
        let wallet = self
            .store
            .wallet
            .find(wallet_id)
            .await?
            .ok_or_else(|| DataError::NotFound(format!("Wallet {wallet_id} not found")))?;
        let asset = wallet.asset.ok_or_else(|| {
            DataError::Inconsistency(format!(
                "Wallet {wallet_id} is missing asset metadata for invoice validation"
            ))
        })?;
        if asset.protocol == Protocol::Bitcoin && asset.asset_ref == NATIVE_ASSET_REF && asset.network == self.network {
            return Ok(());
        }

        Err(DataError::Validation(format!(
            "Wallet {wallet_id} holds {} on {}, but invoice creation requires native BTC on {}",
            asset.display_ticker, asset.network, self.network
        ))
        .into())
    }
}

#[async_trait]
impl InvoiceUseCases for InvoiceService {
    async fn invoice(
        &self,
        wallet_id: Uuid,
        amount: u64,
        description: Option<String>,
        expiry: Option<u32>,
    ) -> Result<Invoice, ApplicationError> {
        debug!(%wallet_id, "Generating invoice");

        self.ensure_wallet_network(wallet_id).await?;

        let invoice_id = Uuid::new_v4();
        let mut invoice = self
            .ln_client
            .invoice(
                amount,
                description.unwrap_or(DEFAULT_INVOICE_DESCRIPTION.to_string()),
                invoice_id.to_string(),
                expiry.unwrap_or(self.invoice_expiry),
                false,
            )
            .await?;
        invoice.id = invoice_id;
        invoice.wallet_id.clone_from(&wallet_id);

        let invoice = self.store.invoice.insert(invoice).await?;

        info!(id = %invoice.id, "Invoice generated successfully");
        Ok(invoice)
    }

    async fn get(&self, id: Uuid) -> Result<Invoice, ApplicationError> {
        trace!(%id, "Fetching invoice");

        let invoice = self
            .store
            .invoice
            .find(id)
            .await?
            .ok_or_else(|| DataError::NotFound("Invoice not found.".to_string()))?;

        debug!(%id, "Invoice fetched successfully");
        Ok(invoice)
    }

    async fn list(&self, filter: InvoiceFilter) -> Result<Vec<Invoice>, ApplicationError> {
        trace!(?filter, "Listing invoices");

        let invoices = self.store.invoice.find_many(filter.clone()).await?;

        debug!(?filter, "Invoices listed successfully");
        Ok(invoices)
    }

    async fn delete(&self, id: Uuid) -> Result<(), ApplicationError> {
        debug!(%id, "Deleting invoice");

        let n_deleted = self
            .store
            .invoice
            .delete_many(InvoiceFilter {
                ids: Some(vec![id]),
                ..Default::default()
            })
            .await?;

        if n_deleted == 0 {
            return Err(DataError::NotFound("Invoice not found.".to_string()).into());
        }

        info!(%id, "Invoice deleted successfully");
        Ok(())
    }

    async fn delete_many(&self, filter: InvoiceFilter) -> Result<u64, ApplicationError> {
        debug!(?filter, "Deleting invoices");

        let n_deleted = self.store.invoice.delete_many(filter.clone()).await?;

        info!(?filter, n_deleted, "Invoices deleted successfully");
        Ok(n_deleted)
    }

    async fn sync(&self) -> Result<u32, ApplicationError> {
        trace!("Synchronizing pending and expired invoices...");

        let pending_invoices = self
            .store
            .invoice
            .find_many(InvoiceFilter {
                status: Some(InvoiceStatus::Pending),
                ledger: Some(Ledger::Lightning),
                ..Default::default()
            })
            .await?;

        // We have to also check the expired invoices because they can become expired while the app is down and the payment received
        // Ideally the expired invoices should be cleaned, not to have too many to sync on startup
        let expired_invoices = self
            .store
            .invoice
            .find_many(InvoiceFilter {
                status: Some(InvoiceStatus::Expired),
                ledger: Some(Ledger::Lightning),
                ..Default::default()
            })
            .await?;

        let invoices = pending_invoices.into_iter().chain(expired_invoices);

        let mut synced = 0;

        for invoice in invoices {
            let Some(ln_invoice) = invoice.ln_invoice.as_ref() else {
                debug!(invoice_id = %invoice.id, "Missing lightning invoice details; skipping sync");
                continue;
            };
            let payment_hash = ln_invoice.payment_hash.clone();
            let Some(node_invoice) = self.ln_client.invoice_by_hash(payment_hash.clone()).await? else {
                continue;
            };
            if node_invoice.status != InvoiceStatus::Settled {
                continue;
            }

            let payment_time = node_invoice.payment_time.unwrap_or_else(Utc::now);
            let event = LnInvoicePaidEvent {
                payment_hash,
                amount_received_msat: node_invoice.amount_received_msat.unwrap_or_default(),
                fee_msat: node_invoice.fee_msat.unwrap_or_default(),
                payment_time,
            };

            self.events.invoice_paid(event).await?;
            synced += 1;
        }

        debug!(synced, "Pending and expired invoices synchronized successfully");
        Ok(synced)
    }
}

#[cfg(test)]
mod tests {
    use crate::{
        application::{
            composition::MockAppStoreBuilder,
            errors::{DatabaseError, LightningError},
        },
        domains::{asset::Asset, event::MockEventUseCases, invoice::LnInvoice, wallet::Wallet},
        infra::lightning::MockLnClient,
    };

    use super::*;

    const EXPIRY: u32 = 3_600;

    fn service(mut store: MockAppStoreBuilder, ln_client: MockLnClient, events: MockEventUseCases) -> InvoiceService {
        store.wallet.expect_find().returning(|wallet_id| {
            Ok(Some(Wallet {
                id: wallet_id,
                asset: Some(native_btc_asset(BtcNetwork::Regtest)),
                ..Default::default()
            }))
        });
        raw_service(store, ln_client, events)
    }

    fn raw_service(store: MockAppStoreBuilder, ln_client: MockLnClient, events: MockEventUseCases) -> InvoiceService {
        InvoiceService::new(
            store.build(),
            Arc::new(ln_client),
            EXPIRY,
            Arc::new(events),
            BtcNetwork::Regtest,
        )
    }

    fn native_btc_asset(network: BtcNetwork) -> Asset {
        Asset {
            id: Uuid::new_v4(),
            code: "BTC".to_string(),
            name: Some("Bitcoin".to_string()),
            protocol: Protocol::Bitcoin,
            network,
            asset_ref: NATIVE_ASSET_REF.to_string(),
            display_ticker: "BTC".to_string(),
            decimals: 11,
            created_at: Utc::now(),
            updated_at: None,
        }
    }

    fn lightning_invoice(payment_hash: &str, status: InvoiceStatus) -> Invoice {
        Invoice {
            id: Uuid::new_v4(),
            ledger: Ledger::Lightning,
            status,
            ln_invoice: Some(LnInvoice {
                payment_hash: payment_hash.to_string(),
                ..Default::default()
            }),
            ..Default::default()
        }
    }

    mod invoice {
        use super::*;

        mod with_defaults {
            use super::*;

            #[tokio::test]
            async fn requests_node_invoice_and_persists_it() {
                let wallet_id = Uuid::new_v4();

                let mut ln_client = MockLnClient::new();
                ln_client
                    .expect_invoice()
                    .withf(|amount, description, _label, expiry, deschashonly| {
                        *amount == 1_000
                            && description == DEFAULT_INVOICE_DESCRIPTION
                            && *expiry == EXPIRY
                            && !deschashonly
                    })
                    .times(1)
                    .returning(|amount, _, _, _, _| {
                        Ok(Invoice {
                            amount_msat: Some(amount),
                            ..Default::default()
                        })
                    });

                let mut store = MockAppStoreBuilder::new();
                store
                    .invoice
                    .expect_insert()
                    .withf(move |invoice| invoice.wallet_id == wallet_id)
                    .times(1)
                    .returning(Ok);

                let service = service(store, ln_client, MockEventUseCases::new());

                let invoice = service.invoice(wallet_id, 1_000, None, None).await.unwrap();

                assert_eq!(invoice.wallet_id, wallet_id);
            }
        }

        mod when_node_invoice_generation_fails {
            use super::*;

            #[tokio::test]
            async fn propagates_lightning_error() {
                let mut ln_client = MockLnClient::new();
                ln_client
                    .expect_invoice()
                    .times(1)
                    .returning(|_, _, _, _, _| Err(LightningError::Invoice("node down".to_string())));

                let service = service(MockAppStoreBuilder::new(), ln_client, MockEventUseCases::new());

                let err = service.invoice(Uuid::new_v4(), 1_000, None, None).await.unwrap_err();

                assert!(matches!(err, ApplicationError::Lightning(_)));
            }
        }

        mod when_persistence_fails {
            use super::*;

            #[tokio::test]
            async fn propagates_database_error() {
                let mut ln_client = MockLnClient::new();
                ln_client
                    .expect_invoice()
                    .times(1)
                    .returning(|_, _, _, _, _| Ok(Invoice::default()));

                let mut store = MockAppStoreBuilder::new();
                store
                    .invoice
                    .expect_insert()
                    .times(1)
                    .returning(|_| Err(DatabaseError::Insert("boom".to_string())));

                let service = service(store, ln_client, MockEventUseCases::new());

                let err = service.invoice(Uuid::new_v4(), 1_000, None, None).await.unwrap_err();

                assert!(matches!(err, ApplicationError::Database(DatabaseError::Insert(_))));
            }
        }

        mod when_wallet_network_differs_from_the_node {
            use super::*;

            #[tokio::test]
            async fn rejects_before_requesting_an_invoice() {
                let wallet_id = Uuid::new_v4();
                let mut store = MockAppStoreBuilder::new();
                store.wallet.expect_find().times(1).returning(move |_| {
                    Ok(Some(Wallet {
                        id: wallet_id,
                        asset: Some(native_btc_asset(BtcNetwork::Bitcoin)),
                        ..Default::default()
                    }))
                });

                let service = raw_service(store, MockLnClient::new(), MockEventUseCases::new());

                let err = service.invoice(wallet_id, 1_000, None, None).await.unwrap_err();

                assert!(matches!(err, ApplicationError::Data(DataError::Validation(_))));
            }
        }
    }

    mod get {
        use super::*;

        mod when_missing {
            use super::*;

            #[tokio::test]
            async fn returns_not_found() {
                let mut store = MockAppStoreBuilder::new();
                store.invoice.expect_find().times(1).returning(|_| Ok(None));

                let service = service(store, MockLnClient::new(), MockEventUseCases::new());

                let err = service.get(Uuid::new_v4()).await.unwrap_err();

                assert!(matches!(err, ApplicationError::Data(DataError::NotFound(_))));
            }
        }
    }

    mod delete {
        use super::*;

        mod when_nothing_is_removed {
            use super::*;

            #[tokio::test]
            async fn returns_not_found() {
                let mut store = MockAppStoreBuilder::new();
                store.invoice.expect_delete_many().times(1).returning(|_| Ok(0));

                let service = service(store, MockLnClient::new(), MockEventUseCases::new());

                let err = service.delete(Uuid::new_v4()).await.unwrap_err();

                assert!(matches!(err, ApplicationError::Data(DataError::NotFound(_))));
            }
        }
    }

    mod sync {
        use super::*;

        mod when_a_pending_invoice_is_settled_on_the_node {
            use super::*;

            #[tokio::test]
            async fn fires_invoice_paid_event_and_counts_it() {
                let mut store = MockAppStoreBuilder::new();
                store.invoice.expect_find_many().times(2).returning(|filter| {
                    if filter.status == Some(InvoiceStatus::Pending) {
                        Ok(vec![lightning_invoice("ph1", InvoiceStatus::Pending)])
                    } else {
                        Ok(vec![])
                    }
                });

                let mut ln_client = MockLnClient::new();
                ln_client
                    .expect_invoice_by_hash()
                    .withf(|payment_hash| payment_hash == "ph1")
                    .times(1)
                    .returning(|_| {
                        Ok(Some(Invoice {
                            status: InvoiceStatus::Settled,
                            amount_received_msat: Some(2_000),
                            fee_msat: Some(1),
                            ..Default::default()
                        }))
                    });

                let mut events = MockEventUseCases::new();
                events
                    .expect_invoice_paid()
                    .withf(|event| event.payment_hash == "ph1" && event.amount_received_msat == 2_000)
                    .times(1)
                    .returning(|_| Ok(()));

                let service = service(store, ln_client, events);

                assert_eq!(service.sync().await.unwrap(), 1);
            }
        }

        mod when_invoice_has_no_lightning_details {
            use super::*;

            #[tokio::test]
            async fn skips_without_querying_the_node() {
                let mut store = MockAppStoreBuilder::new();
                store.invoice.expect_find_many().times(2).returning(|filter| {
                    if filter.status == Some(InvoiceStatus::Pending) {
                        Ok(vec![Invoice {
                            ledger: Ledger::Lightning,
                            ln_invoice: None,
                            ..Default::default()
                        }])
                    } else {
                        Ok(vec![])
                    }
                });

                // invoice_by_hash and invoice_paid are intentionally not expected.
                let service = service(store, MockLnClient::new(), MockEventUseCases::new());

                assert_eq!(service.sync().await.unwrap(), 0);
            }
        }

        mod when_there_is_nothing_pending {
            use super::*;

            #[tokio::test]
            async fn returns_zero() {
                let mut store = MockAppStoreBuilder::new();
                store.invoice.expect_find_many().times(2).returning(|_| Ok(vec![]));

                let service = service(store, MockLnClient::new(), MockEventUseCases::new());

                assert_eq!(service.sync().await.unwrap(), 0);
            }
        }
    }
}
