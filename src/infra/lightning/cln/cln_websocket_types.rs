use chrono::{TimeZone, Utc};
use serde::Deserialize;
use serde_bolt::bitcoin::hashes::hex::ToHex;
use serde_bolt::bitcoin::hashes::{sha256, Hash};

use crate::domains::event::{LnInvoicePaidEvent, LnPayFailureEvent, LnPaySuccessEvent};

#[derive(Debug, Deserialize)]
pub struct InvoicePayment {
    pub preimage: String,
    pub msat: u64,
}

#[derive(Debug, Deserialize)]
pub struct SendPaySuccess {
    pub amount_msat: u64,
    pub amount_sent_msat: u64,
    pub payment_hash: String,
    pub payment_preimage: String,
    pub completed_at: u64,
    pub status: String,
}

#[derive(Debug, Deserialize)]
pub struct SendPayFailure {
    pub message: String,
    pub data: SendPayFailureData,
}

#[derive(Debug, Deserialize)]
pub struct SendPayFailureData {
    pub payment_hash: String,
    pub status: String,
}

impl From<InvoicePayment> for LnInvoicePaidEvent {
    fn from(val: InvoicePayment) -> Self {
        let preimage = hex::decode(val.preimage.clone()).expect("should be hex string");
        let payment_hash = sha256::Hash::hash(&preimage).to_hex();
        LnInvoicePaidEvent {
            payment_hash,
            amount_received_msat: val.msat,
            fee_msat: 0,
            payment_time: Utc::now(),
        }
    }
}

impl From<SendPaySuccess> for LnPaySuccessEvent {
    fn from(val: SendPaySuccess) -> Self {
        LnPaySuccessEvent {
            amount_msat: val.amount_msat,
            fees_msat: val.amount_sent_msat - val.amount_msat,
            payment_hash: val.payment_hash,
            payment_preimage: val.payment_preimage,
            payment_time: Utc.timestamp_opt(val.completed_at as i64, 0).unwrap(),
        }
    }
}

impl From<SendPayFailure> for LnPayFailureEvent {
    fn from(val: SendPayFailure) -> Self {
        LnPayFailureEvent {
            reason: val.message,
            payment_hash: val.data.payment_hash,
        }
    }
}
