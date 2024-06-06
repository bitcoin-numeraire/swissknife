use std::{sync::Arc, time::Duration};

use serde_bolt::bitcoin::hashes::hex::ToHex;
use tokio::{task, time::sleep};
use tonic::transport::Channel;
use tracing::{debug, error, info, trace, warn};

use crate::{
    application::errors::LightningError,
    domains::lightning::services::LnEventsUseCases,
    infra::lightning::cln::cln::{
        listinvoices_request::ListinvoicesIndex, waitanyinvoice_response::WaitanyinvoiceStatus,
        ListinvoicesRequest,
    },
};

use super::cln::{node_client::NodeClient, WaitanyinvoiceRequest};

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
}

impl ClnGrpcListener {
    pub async fn listen_invoices(
        self,
        mut client: NodeClient<Channel>,
    ) -> Result<(), LightningError> {
        let last_updated_invoice = client
            .list_invoices(ListinvoicesRequest {
                index: Some(ListinvoicesIndex::Updated as i32),
                limit: Some(1),
                ..Default::default()
            })
            .await
            .map_err(|e| LightningError::Invoice(e.to_string()))?
            .into_inner()
            .invoices
            .into_iter()
            .next();

        println!("last_updated_invoice: {:?}", last_updated_invoice);

        let mut lastpay_index = match last_updated_invoice {
            Some(invoice) => invoice.pay_index,
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

                        println!("invoice: {:?}", invoice);

                        match invoice.status() {
                            WaitanyinvoiceStatus::Paid => {
                                trace!("New InvoicePaid event received");

                                // let payments_processor = self.payments_processor.clone();
                                tokio::spawn(async move {
                                    println!("Here is where we do the job")
                                    /*if let Err(err) =
                                        payments_processor.incoming_payment(payment).await
                                    {
                                        warn!(%err, "Failed to process incoming payment");
                                    }*/
                                });

                                lastpay_index = Some(invoice.pay_index());
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
