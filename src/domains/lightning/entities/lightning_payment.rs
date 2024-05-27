use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use strum_macros::{Display, EnumString};
use uuid::Uuid;

#[derive(Clone, Debug, Default, Serialize)]
pub struct LightningPayment {
    pub id: Uuid,
    pub user_id: String,
    pub lightning_address: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub payment_hash: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error: Option<String>,
    pub amount_msat: u64,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub fee_msat: Option<u64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub payment_time: Option<DateTime<Utc>>,
    pub status: LightningPaymentStatus,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub metadata: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub success_action: Option<Value>,
    pub created_at: DateTime<Utc>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub updated_at: Option<DateTime<Utc>>,
}

#[derive(Clone, Debug, EnumString, Display, Deserialize, Serialize, PartialEq, Eq, Default)]
pub enum LightningPaymentStatus {
    #[default]
    PENDING,
    SETTLED,
    FAILED,
}

#[derive(Clone, Debug, Deserialize, Serialize, Default)]
pub struct LightningPaymentFilter {
    pub user_id: Option<String>,
    pub status: Option<LightningPaymentStatus>,
    pub limit: Option<u64>,
    pub offset: Option<u64>,
    pub id: Option<Uuid>,
}
