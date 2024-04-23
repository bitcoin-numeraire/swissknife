use chrono::{DateTime, FixedOffset};
use serde::Serialize;
use uuid::Uuid;

#[derive(Clone, Debug, Serialize, Default)]
pub struct LightningPayment {
    pub id: Uuid,
    pub lightning_address: Option<String>,
    pub payment_hash: Option<String>,
    pub error: Option<String>,
    pub amount_msat: u64,
    pub fee_msat: Option<u64>,
    pub payment_time: Option<i64>,
    pub status: String, // TODO: Use enum
    pub description: Option<String>,
    pub metadata: Option<String>,
    pub created_at: DateTime<FixedOffset>,
    pub updated_at: Option<DateTime<FixedOffset>>,
}
