use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use strum_macros::{Display, EnumString};
use uuid::Uuid;

use super::pagination::PaginationFilter;

#[derive(Clone, Debug, Default, Serialize)]
pub struct Payment {
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
    pub payment_type: PaymentType,
    pub status: PaymentStatus,
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
pub enum PaymentStatus {
    #[default]
    PENDING,
    SETTLED,
    FAILED,
}

#[derive(Clone, Debug, EnumString, Display, Deserialize, Serialize, PartialEq, Eq, Default)]
pub enum PaymentType {
    #[default]
    LIGHTNING,
    INTERNAL,
    ONCHAIN,
}

#[derive(Clone, Debug, Deserialize, Serialize, Default)]
pub struct PaymentFilter {
    #[serde(flatten)]
    pub pagination: PaginationFilter,
    pub id: Option<Uuid>,
    pub user_id: Option<String>,
    pub status: Option<PaymentStatus>,
}
