use std::{sync::Arc, time::Duration};

use ::hex::decode;
use chrono::{TimeZone, Utc};
use serde_bolt::bitcoin::hashes::hex::ToHex;
use tokio::{task, time::sleep};
use tonic::transport::Channel;
use tracing::{debug, error, info, trace, warn};

use crate::{
    application::errors::{ApplicationError, LightningError},
    domains::lightning::{entities::LnInvoicePaidEvent, services::LnEventsUseCases},
    infra::lightning::cln::cln::{
        waitanyinvoice_response::WaitanyinvoiceStatus, ListinvoicesRequest,
    },
};

use super::cln::{node_client::NodeClient, WaitanyinvoiceRequest, WaitanyinvoiceResponse};

pub struct ClnGrpcListener {
    ln_events: Arc<dyn LnEventsUseCases>,
    retry_delay: Duration,
}

impl ClnGrpcListener {
    pub fn new(ln_events: Arc<dyn LnEventsUseCases>, retry_delay: Duration) -> Self {
        Self {
            ln_events,
            retry_delay,
        }
    }

    pub async fn listen_invoices(
        self,
        mut client: NodeClient<Channel>,
    ) -> Result<(), ApplicationError> {
        // Temporary. get the latest settled invoice payment_hash as starting point for event listening
        let last_settled_invoice = self.ln_events.latest_settled_invoice().await?;
        let mut lastpay_index = match last_settled_invoice {
            Some(invoice) => {
                let payment_hash = invoice.lightning.unwrap().payment_hash;
                debug!(
                    %payment_hash,
                    "Fetching latest settled invoice from node..."
                );

                let invoices = client
                    .list_invoices(ListinvoicesRequest {
                        payment_hash: decode(payment_hash).ok(),
                        ..Default::default()
                    })
                    .await
                    .map_err(|e| LightningError::ListInvoices(e.to_string()))?
                    .into_inner()
                    .invoices;

                invoices
                    .into_iter()
                    .next()
                    .map(|invoice| invoice.pay_index())
            }
            None => Some(0),
        };

        task::spawn(async move {
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

                                let ln_events = self.ln_events.clone();
                                let event: LnInvoicePaidEvent = invoice.clone().into();

                                loop {
                                    match ln_events.invoice_paid(event.clone()).await {
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
                            WaitanyinvoiceStatus::Expired => {
                                info!(
                                    payment_hash = invoice.payment_hash.to_hex(),
                                    "New InvoiceExpired event received"
                                );
                            }
                        }
                    }
                    Err(err) => {
                        error!(?err, "Error waiting for invoice. Retrying...");
                        sleep(self.retry_delay).await;
                    }
                }
            }
        });

        debug!("Invoice listener started");
        Ok(())
    }
}

impl Into<LnInvoicePaidEvent> for WaitanyinvoiceResponse {
    fn into(self) -> LnInvoicePaidEvent {
        LnInvoicePaidEvent {
            payment_hash: self.payment_hash.to_hex(),
            amount_msat: self.amount_received_msat.as_ref().unwrap().msat,
            fee_msat: 0,
            payment_time: Utc.timestamp_opt(self.paid_at() as i64, 0).unwrap(),
        }
    }
}
