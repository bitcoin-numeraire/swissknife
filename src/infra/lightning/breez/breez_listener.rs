use std::sync::Arc;

use async_trait::async_trait;
use breez_sdk_core::{BreezEvent, EventListener};
use tracing::{trace, warn};

use crate::domains::lightning::usecases::BreezPaymentsProcessorUseCases;

pub struct BreezListener {
    pub payments_processor: Arc<dyn BreezPaymentsProcessorUseCases>,
}

impl BreezListener {
    pub fn new(payments_processor: Arc<dyn BreezPaymentsProcessorUseCases>) -> Self {
        Self { payments_processor }
    }
}

#[async_trait]
impl EventListener for BreezListener {
    fn on_event(&self, e: BreezEvent) {
        match e {
            BreezEvent::InvoicePaid { details } => {
                trace!("New InvoicePaid event received");

                if let Some(payment) = details.payment {
                    let payments_processor = self.payments_processor.clone();
                    tokio::spawn(async move {
                        if let Err(err) = payments_processor.process_incoming_payment(payment).await
                        {
                            warn!(%err, "Failed to process incoming payment");
                        }
                    });
                } else {
                    warn!("Missing payment details from invoice");
                }
            }
            BreezEvent::PaymentSucceed { details } => {
                trace!("New PaymentSucceed event received");

                let payments_processor = self.payments_processor.clone();
                tokio::spawn(async move {
                    if let Err(err) = payments_processor.process_outgoing_payment(details).await {
                        warn!(%err, "Failed to process outgoing payment");
                    }
                });
            }
            BreezEvent::PaymentFailed { details } => {
                trace!("New PaymentFailed event received");

                let payments_processor = self.payments_processor.clone();
                tokio::spawn(async move {
                    if let Err(err) = payments_processor.process_failed_payment(details).await {
                        warn!(%err, "Failed to process outgoing payment");
                    }
                });
            }
            _ => {
                trace!(event = ?e, "New event received");
            }
        }
    }
}
