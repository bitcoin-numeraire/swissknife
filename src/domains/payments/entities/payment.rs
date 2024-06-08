use breez_sdk_core::SuccessActionProcessed;
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use strum_macros::{Display, EnumString};
use uuid::Uuid;

use crate::application::entities::{Ledger, PaginationFilter};

#[derive(Clone, Debug, Default, Serialize)]
pub struct Payment {
    pub id: Uuid,
    pub user_id: String,
    pub ln_address: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub payment_hash: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error: Option<String>,
    pub amount_msat: u64,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub fee_msat: Option<u64>,
    pub ledger: Ledger,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub payment_time: Option<DateTime<Utc>>,
    pub status: PaymentStatus,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub metadata: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub success_action: Option<SuccessActionProcessed>,
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

#[derive(Clone, Debug, Deserialize, Serialize, Default)]
pub struct PaymentFilter {
    #[serde(flatten)]
    pub pagination: PaginationFilter,
    pub id: Option<Uuid>,
    pub user_id: Option<String>,
    pub status: Option<PaymentStatus>,
}
