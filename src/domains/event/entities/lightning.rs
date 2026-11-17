use chrono::{DateTime, Utc};
use uuid::Uuid;

#[derive(Debug, Clone)]
pub struct LnInvoicePaidEvent {
    pub payment_hash: String,
    pub amount_received_msat: u64,
    pub fee_msat: u64,
    pub payment_time: DateTime<Utc>,
}

#[derive(Debug, Clone)]
pub struct LnPaySuccessEvent {
    pub attempt_id: Option<Uuid>,
    pub amount_msat: u64,
    pub fees_msat: u64,
    pub payment_hash: String,
    pub payment_preimage: String,
    pub payment_time: DateTime<Utc>,
}

#[derive(Debug, Clone)]
pub struct LnPayFailureEvent {
    pub attempt_id: Option<Uuid>,
    pub reason: String,
    pub payment_hash: String,
}
