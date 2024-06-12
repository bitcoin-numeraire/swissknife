use chrono::{TimeZone, Utc};
use serde::Deserialize;

use crate::domains::lightning::entities::{
    LnInvoicePaidEvent, LnPayFailureEvent, LnPaySuccessEvent,
};

#[derive(Debug, Deserialize)]
pub struct CoinMovement {
    pub credit_msat: u64,
    pub payment_hash: String,
    pub timestamp: u64,
    #[serde(rename = "type")]
    pub movement_type: String,
    pub tags: Vec<String>,
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

impl Into<LnInvoicePaidEvent> for CoinMovement {
    fn into(self) -> LnInvoicePaidEvent {
        LnInvoicePaidEvent {
            payment_hash: self.payment_hash,
            amount_msat: self.credit_msat,
            fee_msat: 0,
            payment_time: Utc.timestamp_opt(self.timestamp as i64, 0).unwrap(),
        }
    }
}

impl Into<LnPaySuccessEvent> for SendPaySuccess {
    fn into(self) -> LnPaySuccessEvent {
        LnPaySuccessEvent {
            amount_msat: self.amount_msat,
            fees_msat: self.amount_sent_msat - self.amount_msat,
            payment_hash: self.payment_hash,
            payment_preimage: self.payment_preimage,
            payment_time: Utc.timestamp_opt(self.completed_at as i64, 0).unwrap(),
        }
    }
}

impl Into<LnPayFailureEvent> for SendPayFailure {
    fn into(self) -> LnPayFailureEvent {
        LnPayFailureEvent {
            reason: self.message,
            payment_hash: self.data.payment_hash,
        }
    }
}
