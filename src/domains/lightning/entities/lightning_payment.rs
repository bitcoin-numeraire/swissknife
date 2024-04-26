use chrono::{DateTime, FixedOffset};
use serde_json::Value;
use strum_macros::{Display, EnumString};
use uuid::Uuid;

#[derive(Clone, Debug, Default)]
pub struct LightningPayment {
    pub id: Uuid,
    pub lightning_address: Option<String>,
    pub payment_hash: Option<String>,
    pub error: Option<String>,
    pub amount_msat: u64,
    pub fee_msat: Option<u64>,
    pub payment_time: Option<i64>,
    pub status: LightningPaymentStatus,
    pub description: Option<String>,
    pub metadata: Option<String>,
    pub success_action: Option<Value>,
    pub created_at: DateTime<FixedOffset>,
    pub updated_at: Option<DateTime<FixedOffset>>,
}

#[derive(Clone, Debug, EnumString, Display, PartialEq, Eq, Default)]
pub enum LightningPaymentStatus {
    #[default]
    PENDING,
    SETTLED,
    FAILED,
}
