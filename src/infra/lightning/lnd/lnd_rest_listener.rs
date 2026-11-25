use std::{io, sync::Arc};

use async_trait::async_trait;
use futures_util::{StreamExt, TryStreamExt};
use reqwest::Client;
use serde_json::Value;
use tokio_util::{
    codec::{FramedRead, LinesCodec},
    io::StreamReader,
};
use tracing::error;

use crate::application::{composition::AppServices, errors::LightningError};
use crate::domains::bitcoin::{BtcTransaction, OnchainSyncCursor};
use crate::infra::lightning::EventsListener;

use super::lnd_types::{InvoiceResponse, TransactionResponse};
use super::{LndRestClient, LndRestClientConfig};

enum Subscription {
    Invoices,
    Transactions,
}

pub struct LndRestListener {
    client: Client,
    host: String,
    services: Arc<AppServices>,
}

impl LndRestListener {
    pub async fn new(config: LndRestClientConfig, services: Arc<AppServices>) -> Result<Self, LightningError> {
        // A subscription can legitimately be idle longer than a unary RPC timeout.
        let client = LndRestClient::http_builder(&config)
            .await?
            .build()
            .map_err(|e| LightningError::ParseConfig(e.to_string()))?;
        Ok(Self {
            client,
            host: config.host,
            services,
        })
    }

    async fn subscribe(&self, subscription: Subscription) -> Result<(), LightningError> {
        let path = match subscription {
            Subscription::Invoices => "invoices",
            Subscription::Transactions => {
                self.services
                    .bitcoin
                    .sync()
                    .await
                    .map_err(|e| LightningError::Listener(e.to_string()))?;
                "transactions"
            }
        };
        let response = self
            .client
            .get(format!("https://{}/v1/{path}/subscribe", self.host))
            .send()
            .await
            .and_then(reqwest::Response::error_for_status)
            .map_err(|e| LightningError::Listener(e.to_string()))?;
        let stream = StreamReader::new(response.bytes_stream().map_err(io::Error::other));
        // grpc-gateway delimits JSON envelopes with newlines. Preserve the previous
        // WebSocket maximum message size while allowing frames split across HTTP chunks.
        let mut lines = FramedRead::new(stream, LinesCodec::new_with_max_length(64 << 20));
        while let Some(line) = lines.next().await {
            let line = line.map_err(|e| LightningError::Listener(e.to_string()))?;
            let value: Value = match serde_json::from_str(&line) {
                Ok(value) => value,
                Err(err) => {
                    error!(%err, %path, "Invalid LND subscription event");
                    continue;
                }
            };
            if let Some(error) = value.get("error") {
                return Err(LightningError::Listener(format!(
                    "LND {path} subscription failed: {error}"
                )));
            }
            if let Some(event) = value.get("result") {
                match subscription {
                    Subscription::Invoices => self.process_invoice(event).await?,
                    Subscription::Transactions => self.process_transaction(event).await?,
                }
            }
        }
        Err(LightningError::Listener(format!("LND {path} subscription closed")))
    }
    async fn process_invoice(&self, event: &Value) -> Result<(), LightningError> {
        match serde_json::from_value::<InvoiceResponse>(event.clone()) {
            Ok(invoice) => {
                if invoice.state.as_str() == "SETTLED" {
                    if let Err(err) = self.services.event.invoice_paid(invoice.into()).await {
                        return Err(LightningError::EventProcessing(err.to_string()));
                    }
                }
            }
            Err(err) => {
                error!(%err, "Failed to parse SubscribeInvoices event");
            }
        }

        Ok(())
    }

    async fn process_transaction(&self, event: &Value) -> Result<(), LightningError> {
        match serde_json::from_value::<TransactionResponse>(event.clone()) {
            Ok(transaction) => {
                let transaction: BtcTransaction = transaction.into();
                self.handle_transaction(transaction).await?;
            }
            Err(err) => {
                error!(%err, "Failed to parse SubscribeTransactions event");
            }
        }

        Ok(())
    }

    async fn handle_transaction(&self, transaction: BtcTransaction) -> Result<(), LightningError> {
        for output in transaction.outputs.iter().filter(|output| output.is_ours) {
            // Don't swallow: a failed write must not advance the cursor, or the deposit is
            // lost. Propagating exits the listener so the supervisor reconnects and `sync()`
            // reruns from the un-advanced cursor, reprocessing idempotently.
            self.services
                .event
                .onchain_deposit(transaction.deposit_event(output))
                .await
                .map_err(|e| LightningError::EventProcessing(e.to_string()))?;
        }

        if transaction.is_outgoing {
            self.services
                .event
                .onchain_withdrawal(transaction.withdrawal_event())
                .await
                .map_err(|e| LightningError::EventProcessing(e.to_string()))?;
        }

        if let Some(block_height) = transaction.block_height.filter(|&h| h > 0) {
            self.services
                .system
                .set_onchain_cursor(OnchainSyncCursor::BlockHeight(block_height))
                .await
                .map_err(|e| LightningError::Listener(e.to_string()))?;
        }

        Ok(())
    }
}

#[async_trait]
impl EventsListener for LndRestListener {
    async fn listen(&self) -> Result<(), LightningError> {
        tokio::try_join!(
            self.subscribe(Subscription::Invoices),
            self.subscribe(Subscription::Transactions)
        )?;
        Ok(())
    }
}
