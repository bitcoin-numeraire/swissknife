use std::sync::Arc;

use anyhow::{anyhow, Result};
use async_trait::async_trait;
use serde_bolt::bitcoin::hashes::hex::ToHex;
use tokio::time::sleep;
use tonic::Code;
use tracing::{error, info, trace, warn};

use crate::{
    application::errors::{ApplicationError, LightningError},
    domains::{bitcoin::BitcoinWallet, event::EventUseCases},
    infra::lightning::EventsListener,
};

use super::{
    cln::{waitanyinvoice_response::WaitanyinvoiceStatus, WaitanyinvoiceRequest},
    cln_grpc_client::{ClnClientConfig, ClnGrpcClient},
};

pub struct ClnGrpcListener {
    config: ClnClientConfig,
    events: Arc<dyn EventUseCases>,
}

impl ClnGrpcListener {
    pub fn new(config: ClnClientConfig, events: Arc<dyn EventUseCases>, _wallet: Arc<dyn BitcoinWallet>) -> Self {
        Self { config, events }
    }

    async fn listen_invoices(&self) -> Result<()> {
        let mut client = ClnGrpcClient::connect(&self.config).await?;
        let retry_delay = self.config.retry_delay;

        let mut lastpay_index = None;

        loop {
            trace!(lastpay_index, "Waiting for new invoice...");

            match client
                .wait_any_invoice(WaitanyinvoiceRequest {
                    lastpay_index,
                    timeout: None,
                })
                .await
            {
                Ok(response) => {
                    let invoice = response.into_inner();

                    match invoice.status() {
                        WaitanyinvoiceStatus::Paid => {
                            trace!("New InvoicePaid event received");

                            loop {
                                match self.events.invoice_paid(invoice.clone().into()).await {
                                    Ok(_) => break,
                                    Err(err) => match err {
                                        ApplicationError::Database(db_err) => {
                                            error!(%db_err, "Database error, retrying...");
                                            sleep(retry_delay).await;
                                        }
                                        _ => {
                                            warn!(%err, "Failed to process incoming payment");
                                            break;
                                        }
                                    },
                                }
                            }
                            lastpay_index = invoice.pay_index;
                        }
                        WaitanyinvoiceStatus::Expired => {
                            info!(
                                payment_hash = invoice.payment_hash.to_hex(),
                                "New InvoiceExpired event received"
                            );
                        }
                    }
                }
                Err(err) => match err.code() {
                    Code::Aborted
                    | Code::Cancelled
                    | Code::DeadlineExceeded
                    | Code::Internal
                    | Code::FailedPrecondition
                    | Code::Unavailable => {
                        error!(err = err.message(), "Error waiting for invoice. Retrying...");
                        sleep(retry_delay).await;
                    }
                    _ => {
                        return Err(anyhow!(err.to_string()));
                    }
                },
            }
        }
    }
}

#[async_trait]
impl EventsListener for ClnGrpcListener {
    async fn listen(
        &self,
        _events: Arc<dyn EventUseCases>,
        _bitcoin_wallet: Arc<dyn BitcoinWallet>,
    ) -> Result<(), LightningError> {
        self.listen_invoices()
            .await
            .map_err(|err| LightningError::Listener(err.to_string()))
    }
}
