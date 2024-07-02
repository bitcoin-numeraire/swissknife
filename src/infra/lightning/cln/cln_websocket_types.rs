use chrono::{TimeZone, Utc};
use serde::Deserialize;

use crate::domains::lightning::entities::{
    LnInvoicePaidEvent, LnPayFailureEvent, LnPaySuccessEvent,
};

#[derive(Debug, Deserialize)]
pub struct CoinMovement {
    pub credit_msat: u64,
    pub payment_hash: Option<String>,
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

impl From<CoinMovement> for LnInvoicePaidEvent {
    fn from(val: CoinMovement) -> Self {
        LnInvoicePaidEvent {
            payment_hash: val.payment_hash.unwrap(),
            amount_msat: val.credit_msat,
            fee_msat: 0,
            payment_time: Utc.timestamp_opt(val.timestamp as i64, 0).unwrap(),
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
