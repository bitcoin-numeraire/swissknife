use std::{sync::Arc, time::Duration};

use async_trait::async_trait;
use chrono::{TimeZone, Utc};
use tokio::time::sleep;
use tonic_lnd::{connect, lnrpc, tonic::Code, tonic::Status, LightningClient};
use tracing::{error, warn};

use crate::{
    application::{entities::AppServices, errors::LightningError},
    domains::{
        bitcoin::{BitcoinWallet, BtcTransaction, BtcTransactionOutput, OnchainSyncCursor},
        event::LnInvoicePaidEvent,
    },
    infra::lightning::EventsListener,
};

use super::LndGrpcClientConfig;

pub struct LndGrpcListener {
    client: LightningClient,
    services: Arc<AppServices>,
    network: crate::domains::bitcoin::BtcNetwork,
    retry_delay: Duration,
}

impl LndGrpcListener {
    pub async fn new(
        config: LndGrpcClientConfig,
        services: Arc<AppServices>,
        wallet: Arc<dyn BitcoinWallet>,
    ) -> Result<Self, LightningError> {
        let mut client = connect(config.endpoint.clone(), &config.cert_path, &config.macaroon_path)
            .await
            .map_err(|e| LightningError::Connect(e.to_string()))?;

        Ok(Self {
            client: client.lightning().clone(),
            services,
            network: wallet.network(),
            retry_delay: config.retry_delay,
        })
    }

    async fn handle_grpc_error(&self, err: Status, context: &str) -> Result<(), LightningError> {
        match err.code() {
            Code::Aborted
            | Code::Cancelled
            | Code::DeadlineExceeded
            | Code::Internal
            | Code::FailedPrecondition
            | Code::Unavailable => {
                error!(err = err.message(), "{}. Retrying...", context);
                sleep(self.retry_delay).await;
                Ok(())
            }
            _ => Err(LightningError::Listener(err.to_string())),
        }
    }

    async fn listen_invoices(&self) -> Result<(), LightningError> {
        loop {
            let result = {
                let mut client = self.client.clone();
                client.subscribe_invoices(lnrpc::InvoiceSubscription::default()).await
            };

            match result {
                Ok(response) => {
                    let mut stream = response.into_inner();
                    while let Some(invoice) = stream
                        .message()
                        .await
                        .map_err(|e| LightningError::Listener(format!("Failed to read invoice stream: {}", e)))?
                    {
                        self.handle_invoice(invoice).await;
                    }
                }
                Err(err) => {
                    self.handle_grpc_error(err, "Error subscribing to invoices").await?;
                }
            }
        }
    }

    async fn listen_transactions(&self) -> Result<(), LightningError> {
        self.services
            .bitcoin
            .sync()
            .await
            .map_err(|e| LightningError::Listener(e.to_string()))?;

        loop {
            let result = {
                let mut client = self.client.clone();
                client
                    .subscribe_transactions(lnrpc::GetTransactionsRequest::default())
                    .await
            };

            match result {
                Ok(response) => {
                    let mut stream = response.into_inner();
                    while let Some(transaction) = stream
                        .message()
                        .await
                        .map_err(|e| LightningError::Listener(format!("Failed to read transaction stream: {}", e)))?
                    {
                        let transaction = Self::map_transaction(transaction);
                        self.handle_transaction(transaction).await;
                    }
                }
                Err(err) => {
                    self.handle_grpc_error(err, "Error subscribing to transactions").await?;
                }
            }
        }
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

    async fn handle_transaction(&self, transaction: BtcTransaction) {
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

            if let Err(err) = result {
                error!(%err, "Failed to process onchain transaction");
            }
        }

        if let Some(block_height) = transaction.block_height.filter(|&h| h > 0) {
            if let Err(err) = self
                .services
                .system
                .set_onchain_cursor(OnchainSyncCursor::BlockHeight(block_height))
                .await
            {
                warn!(%err, "Failed to persist onchain cursor");
            }
        }
    }
}

#[async_trait]
impl EventsListener for LndGrpcListener {
    async fn listen(&self) -> Result<(), LightningError> {
        tokio::try_join!(self.listen_invoices(), self.listen_transactions())?;
        Ok(())
    }
}
