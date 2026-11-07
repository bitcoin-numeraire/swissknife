use chrono::Utc;
use serde::Deserialize;
use serde_bolt::bitcoin::hashes::{sha256, Hash};

use crate::domains::event::LnInvoicePaidEvent;

#[derive(Debug, Deserialize)]
pub struct InvoicePayment {
    pub preimage: String,
    pub msat: u64,
}

impl From<InvoicePayment> for LnInvoicePaidEvent {
    fn from(val: InvoicePayment) -> Self {
        let preimage = hex::decode(val.preimage.clone()).expect("should be hex string");
        let payment_hash = sha256::Hash::hash(&preimage).to_string();
        LnInvoicePaidEvent {
            payment_hash,
            amount_received_msat: val.msat,
            fee_msat: 0,
            payment_time: Utc::now(),
        }
    }
}
