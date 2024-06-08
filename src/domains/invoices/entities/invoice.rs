use std::time::Duration;

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use serde_with::serde_as;
use serde_with::DurationSeconds;
use strum_macros::{Display, EnumString};
use uuid::Uuid;

use crate::application::entities::OrderDirection;
use crate::application::entities::{Currency, Ledger, PaginationFilter};

#[serde_as]
#[derive(Clone, Debug, Default, Serialize)]
pub struct Invoice {
    pub id: Uuid,
    pub user_id: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub ln_address: Option<Uuid>,
    pub description: Option<String>,
    pub currency: Currency,
    pub amount_msat: Option<u64>,
    pub timestamp: DateTime<Utc>,
    pub status: InvoiceStatus,
    pub ledger: Ledger,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub fee_msat: Option<u64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub payment_time: Option<DateTime<Utc>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub label: Option<Uuid>,
    pub created_at: DateTime<Utc>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub updated_at: Option<DateTime<Utc>>,
    #[serde(flatten)]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub lightning: Option<LnInvoice>,
}

#[serde_as]
#[derive(Clone, Debug, Default, Serialize)]
pub struct LnInvoice {
    pub payment_hash: String,
    pub bolt11: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub description_hash: Option<String>,
    pub payee_pubkey: String,
    pub min_final_cltv_expiry_delta: u64,
    pub payment_secret: String,
    #[serde_as(as = "DurationSeconds<u64>")]
    pub expiry: Duration,
    pub expires_at: DateTime<Utc>,
}

#[derive(Clone, Debug, EnumString, Deserialize, Serialize, Display, PartialEq, Eq, Default)]
pub enum InvoiceStatus {
    #[default]
    PENDING,
    SETTLED,
    EXPIRED,
}

#[derive(Clone, Debug, Deserialize, Serialize, Default)]
pub struct InvoiceFilter {
    #[serde(flatten)]
    pub pagination: PaginationFilter,
    pub id: Option<Uuid>,
    pub user_id: Option<String>,
    pub status: Option<InvoiceStatus>,
    pub ledger: Option<Ledger>,
    #[serde(default)]
    pub order_direction: OrderDirection,
}
