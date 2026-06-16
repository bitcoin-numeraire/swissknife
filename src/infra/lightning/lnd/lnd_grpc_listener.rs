use std::sync::Arc;

use async_trait::async_trait;
use chrono::{TimeZone, Utc};
use tracing::warn;

use crate::{
    application::{composition::AppServices, errors::LightningError},
    domains::{
        bitcoin::{BitcoinWallet, BtcTransaction, BtcTransactionOutput, OnchainSyncCursor},
        event::LnInvoicePaidEvent,
    },
    infra::lightning::{
        lnd::{
            lnrpc::{self, lightning_client::LightningClient},
            LndChannel, LndGrpcClient,
        },
        EventsListener,
    },
};

use super::LndGrpcClientConfig;

pub struct LndGrpcListener {
    client: LightningClient<LndChannel>,
    services: Arc<AppServices>,
    network: crate::domains::bitcoin::BtcNetwork,
}

impl LndGrpcListener {
    pub async fn new(
        config: LndGrpcClientConfig,
        services: Arc<AppServices>,
        wallet: Arc<dyn BitcoinWallet>,
    ) -> Result<Self, LightningError> {
        let channel = LndGrpcClient::connect(&config).await?;

        Ok(Self {
            client: LightningClient::new(channel),
            services,
            network: wallet.network(),
        })
    }

    // Both streams propagate every error out of `listen()`; the `EventListener` supervisor
    // owns reconnection (re-running `sync()` from the un-advanced cursor). A clean stream
    // close is also surfaced as an error so it triggers the same reconnect path (issue #267).
    async fn listen_invoices(&self) -> Result<(), LightningError> {
        let response = self
            .client
            .clone()
            .subscribe_invoices(lnrpc::InvoiceSubscription::default())
            .await
            .map_err(|e| LightningError::Listener(e.to_string()))?;

        let mut stream = response.into_inner();
        while let Some(invoice) = stream
            .message()
            .await
            .map_err(|e| LightningError::Listener(format!("Failed to read invoice stream: {}", e)))?
        {
            self.handle_invoice(invoice).await;
        }

        Err(LightningError::Listener("LND invoice stream closed".to_string()))
    }

    async fn listen_transactions(&self) -> Result<(), LightningError> {
        self.services
            .bitcoin
            .sync()
            .await
            .map_err(|e| LightningError::Listener(e.to_string()))?;

        let response = self
            .client
            .clone()
            .subscribe_transactions(lnrpc::GetTransactionsRequest::default())
            .await
            .map_err(|e| LightningError::Listener(e.to_string()))?;

        let mut stream = response.into_inner();
        while let Some(transaction) = stream
            .message()
            .await
            .map_err(|e| LightningError::Listener(format!("Failed to read transaction stream: {}", e)))?
        {
            let transaction = Self::map_transaction(transaction);
            self.handle_transaction(transaction).await?;
        }

        Err(LightningError::Listener("LND transaction stream closed".to_string()))
    }

    async fn handle_invoice(&self, invoice: lnrpc::Invoice) {
        if invoice.state() != lnrpc::invoice::InvoiceState::Settled {
            return;
        }

        if invoice.r_hash.is_empty() {
            warn!("Invoice update missing payment hash");
            return;
        }

        let payment_time = Utc.timestamp_opt(invoice.settle_date, 0).unwrap();
        let event = LnInvoicePaidEvent {
            payment_hash: hex::encode(invoice.r_hash),
            amount_received_msat: invoice.amt_paid_msat as u64,
            fee_msat: 0,
            payment_time,
        };

        if let Err(err) = self.services.event.invoice_paid(event).await {
            warn!(%err, "Failed to process incoming payment");
        }
    }

    fn map_transaction(transaction: lnrpc::Transaction) -> BtcTransaction {
        let is_outgoing = transaction
            .previous_outpoints
            .iter()
            .any(|outpoint| outpoint.is_our_output);

        let outputs = transaction
            .output_details
            .into_iter()
            .filter_map(|detail| {
                let output_index = u32::try_from(detail.output_index).ok()?;
                let amount = u64::try_from(detail.amount).ok()?;
                Some(BtcTransactionOutput {
                    output_index,
                    address: detail.address,
                    amount_sat: amount,
                    is_ours: detail.is_our_address,
                })
            })
            .collect();

        let block_height = if transaction.block_height > 0 {
            Some(transaction.block_height as u32)
        } else {
            None
        };

        BtcTransaction {
            txid: transaction.tx_hash,
            block_height,
            outputs,
            is_outgoing,
        }
    }

    async fn handle_transaction(&self, transaction: BtcTransaction) -> Result<(), LightningError> {
        let relevant_outputs = transaction.outputs.iter().filter(|output| {
            if transaction.is_outgoing {
                !output.is_ours
            } else {
                output.is_ours
            }
        });

        for output in relevant_outputs {
            let result = if transaction.is_outgoing {
                self.services
                    .event
                    .onchain_withdrawal(transaction.withdrawal_event())
                    .await
            } else {
                self.services
                    .event
                    .onchain_deposit(transaction.deposit_event(output), self.network.into())
                    .await
            };

            // Propagate write failures instead of swallowing them. The cursor must not
            // advance past an event we failed to persist or the deposit is lost; bubbling
            // the error exits the listener so the supervisor reconnects and `sync()` reruns
            // from the un-advanced cursor, reprocessing idempotently.
            result.map_err(|e| LightningError::EventProcessing(e.to_string()))?;
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
impl EventsListener for LndGrpcListener {
    async fn listen(&self) -> Result<(), LightningError> {
        tokio::try_join!(self.listen_invoices(), self.listen_transactions())?;
        Ok(())
    }
}
