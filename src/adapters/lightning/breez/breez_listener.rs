use std::sync::Arc;

use async_trait::async_trait;
use breez_sdk_core::{BreezEvent, EventListener};
use tracing::{trace, warn};

use crate::domains::lightning::usecases::LightningPaymentsUseCases;

pub struct BreezListener {
    pub payments_processor: Arc<dyn LightningPaymentsUseCases>,
}

impl BreezListener {
    pub fn new(payments_processor: Arc<dyn LightningPaymentsUseCases>) -> Self {
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
                            warn!(err = err.to_string(), "Failed to process payment");
                        }
                    });
                } else {
                    warn!("Missing payment details from invoice");
                }
            }
            _ => {}
        }
    }
}
