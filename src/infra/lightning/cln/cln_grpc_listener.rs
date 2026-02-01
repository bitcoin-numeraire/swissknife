use std::{sync::Arc, time::Duration};

use anyhow::anyhow;
use async_trait::async_trait;
use tokio::{sync::Mutex, time::sleep};
use tonic::{transport::Channel, Code};
use tracing::{error, trace, warn};

use crate::{
    application::errors::{ApplicationError, LightningError},
    domains::{bitcoin::BitcoinWallet, event::EventUseCases},
    infra::lightning::EventsListener,
};

use super::{
    cln::{node_client::NodeClient, waitanyinvoice_response::WaitanyinvoiceStatus, WaitanyinvoiceRequest},
    cln_grpc_client::{ClnClientConfig, ClnGrpcClient},
};

pub struct ClnGrpcListener {
    client: Mutex<NodeClient<Channel>>,
    events: Arc<dyn EventUseCases>,
    retry_delay: Duration,
}

impl ClnGrpcListener {
    pub async fn new(
        config: ClnClientConfig,
        events: Arc<dyn EventUseCases>,
        _wallet: Arc<dyn BitcoinWallet>,
    ) -> Result<Self, LightningError> {
        let client = ClnGrpcClient::connect(&config).await?;

        Ok(Self {
            client: Mutex::new(client),
            events,
            retry_delay: config.retry_delay,
        })
    }
}

#[async_trait]
impl EventsListener for ClnGrpcListener {
    async fn listen(&self) -> Result<(), LightningError> {
        let mut lastpay_index = None;

        loop {
            trace!(lastpay_index, "Waiting for new invoice...");

            let result = {
                let mut client = self.client.lock().await;
                client
                    .wait_any_invoice(WaitanyinvoiceRequest {
                        lastpay_index,
                        timeout: None,
                    })
                    .await
            };

            match result {
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
                                            sleep(self.retry_delay).await;
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
                        WaitanyinvoiceStatus::Expired => {}
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
                        sleep(self.retry_delay).await;
                    }
                    _ => {
                        return Err(LightningError::Listener(anyhow!(err.to_string()).to_string()));
                    }
                },
            }
        }
    }
}
