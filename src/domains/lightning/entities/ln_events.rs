use chrono::{DateTime, Utc};

#[derive(Debug, Clone)]
pub struct LnInvoicePaidEvent {
    pub payment_hash: String,
    pub amount_msat: u64,
    pub fee_msat: u64,
    pub payment_time: DateTime<Utc>,
}

#[derive(Debug, Clone)]
pub struct LnPaySuccessEvent {
    pub amount_msat: u64,
    pub fees_msat: u64,
    pub payment_hash: String,
    pub payment_preimage: String,
    pub payment_time: DateTime<Utc>,
}

#[derive(Debug, Clone)]
pub struct LnPayFailureEvent {
    pub reason: String,
    pub payment_hash: String,
}
