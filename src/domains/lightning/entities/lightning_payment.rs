use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use strum_macros::{Display, EnumString};
use uuid::Uuid;

#[derive(Clone, Debug, Default)]
pub struct LightningPayment {
    pub id: Uuid,
    pub user_id: String,
    pub lightning_address: Option<String>,
    pub payment_hash: Option<String>,
    pub error: Option<String>,
    pub amount_msat: u64,
    pub fee_msat: Option<u64>,
    pub payment_time: Option<DateTime<Utc>>,
    pub status: LightningPaymentStatus,
    pub description: Option<String>,
    pub metadata: Option<String>,
    pub success_action: Option<Value>,
    pub created_at: DateTime<Utc>,
    pub updated_at: Option<DateTime<Utc>>,
}

#[derive(Clone, Debug, EnumString, Display, Deserialize, Serialize, PartialEq, Eq, Default)]
pub enum LightningPaymentStatus {
    #[default]
    PENDING,
    SETTLED,
    FAILED,
}
