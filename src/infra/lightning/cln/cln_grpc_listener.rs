use std::{sync::Arc, time::Duration};

use anyhow::{anyhow, Result};
use serde_bolt::bitcoin::hashes::hex::ToHex;
use tokio::time::sleep;
use tonic::{transport::Channel, Code};
use tracing::{debug, error, info, trace, warn};

use crate::{
    application::errors::ApplicationError,
    domains::ln_node::LnEventsUseCases,
    infra::lightning::cln::cln::{waitanyinvoice_response::WaitanyinvoiceStatus, ListinvoicesRequest},
};

use super::cln::{node_client::NodeClient, WaitanyinvoiceRequest};

pub async fn listen_invoices(
    mut client: NodeClient<Channel>,
    ln_events: Arc<dyn LnEventsUseCases>,
    retry_delay: Duration,
) -> Result<()> {
    // Temporary. get the latest settled invoice payment_hash as starting point for event listening
    let last_settled_invoice = ln_events.latest_settled_invoice().await?;

    let mut lastpay_index = match last_settled_invoice {
        Some(invoice) => {
            debug!(
                id = %invoice.id,
                "Fetching latest settled invoice from node..."
            );

            let invoices = client
                .list_invoices(ListinvoicesRequest {
                    label: Some(invoice.id.into()),
                    ..Default::default()
                })
                .await?
                .into_inner()
                .invoices;

            invoices.into_iter().next().map(|invoice| invoice.pay_index())
        }
        None => Some(0),
    };

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
                            match ln_events.invoice_paid(invoice.clone().into()).await {
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
