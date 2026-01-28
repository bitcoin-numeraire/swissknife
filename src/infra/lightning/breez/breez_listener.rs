use async_trait::async_trait;
use breez_sdk_core::{BreezEvent, EventListener, PaymentStatus, PaymentType};
use tracing::{trace, warn};

use crate::domains::event::EventService;

pub struct BreezListener {
    pub events: EventService,
}

impl BreezListener {
    pub fn new(events: EventService) -> Self {
        Self { events }
    }
}

#[async_trait]
impl EventListener for BreezListener {
    fn on_event(&self, e: BreezEvent) {
        match e {
            BreezEvent::InvoicePaid { details } => {
                trace!("New InvoicePaid event received");

                if let Some(payment) = details.payment {
                    if payment.status != PaymentStatus::Complete {
                        warn!(payment_hash = payment.id, "Invalid payment status. Expected Complete.");
                        return;
                    }

                    if payment.payment_type != PaymentType::Received {
                        warn!(payment_hash = payment.id, "Invalid payment type. Expected Received.");
                        return;
                    }

                    let ln_events = self.events.clone();
                    tokio::spawn(async move {
                        if let Err(err) = ln_events.invoice_paid(payment.into()).await {
                            warn!(%err, "Failed to process incoming payment");
                        }
                    });
                } else {
                    warn!("Missing payment details from invoice");
                }
            }
            BreezEvent::PaymentSucceed { details } => {
                trace!("New PaymentSucceed event received");

                if details.status != PaymentStatus::Complete {
                    warn!(payment_hash = details.id, "Invalid payment status. Expected Complete.");
                    return;
                }

                if details.payment_type != PaymentType::Sent {
                    warn!(payment_hash = details.id, "Invalid payment type. Expected Sent.");
                    return;
                }

                let ln_events = self.events.clone();
                tokio::spawn(async move {
                    if let Err(err) = ln_events.outgoing_payment(details.into()).await {
                        warn!(%err, "Failed to process outgoing payment");
                    }
                });
            }
            BreezEvent::PaymentFailed { details } => {
                trace!("New PaymentFailed event received");

                let ln_events = self.events.clone();
                tokio::spawn(async move {
                    if let Err(err) = ln_events.failed_payment(details.into()).await {
                        warn!(%err, "Failed to process outgoing payment");
                    }
                });
            }
            _ => {}
        }
    }
}
