use breez_sdk_liquid::prelude::{EventListener, PaymentType, SdkEvent};
use tracing::{trace, warn};

use crate::domains::event::{EventService, EventUseCases};

pub struct BreezListener {
    pub events: EventService,
}

impl BreezListener {
    pub fn new(events: EventService) -> Self {
        Self { events }
    }
}

impl EventListener for BreezListener {
    fn on_event(&self, e: SdkEvent) {
        match e {
            SdkEvent::PaymentSucceeded { details } => {
                trace!("New PaymentSucceeded event received");

                match details.payment_type {
                    PaymentType::Receive => {
                        let ln_events = self.events.clone();
                        tokio::spawn(async move {
                            if let Err(err) = ln_events.invoice_paid(details.into()).await {
                                warn!(%err, "Failed to process incoming payment");
                            }
                        });
                    }
                    PaymentType::Send => {
                        let ln_events = self.events.clone();
                        tokio::spawn(async move {
                            if let Err(err) = ln_events.outgoing_payment(details.into()).await {
                                warn!(%err, "Failed to process outgoing payment");
                            }
                        });
                    }
                }
            }
            SdkEvent::PaymentFailed { details } => {
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
