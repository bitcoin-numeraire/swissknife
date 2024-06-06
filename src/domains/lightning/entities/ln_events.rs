use chrono::{DateTime, Utc};

#[derive(Debug, Clone)]
pub struct LnInvoicePaidEvent {
    pub payment_hash: String,
    pub amount_msat: u64,
    pub fee_msat: u64,
    pub payment_time: DateTime<Utc>,
}
