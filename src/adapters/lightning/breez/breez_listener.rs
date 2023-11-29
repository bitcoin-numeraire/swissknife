use async_trait::async_trait;
use breez_sdk_core::{BreezEvent, EventListener};

pub struct BreezListener {}

#[async_trait]
impl EventListener for BreezListener {
    fn on_event(&self, e: BreezEvent) {
        match e {
            BreezEvent::InvoicePaid { details } => {
                println!("Invoice received: {:?}", details.payment_hash);
            }
            _ => {}
        }
    }
}
