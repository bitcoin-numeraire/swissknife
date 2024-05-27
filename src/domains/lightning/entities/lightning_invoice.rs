use std::time::Duration;

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use serde_with::serde_as;
use serde_with::DurationSeconds;
use strum_macros::{Display, EnumString};
use uuid::Uuid;

#[serde_as]
#[derive(Clone, Debug, Default, Serialize)]
pub struct LightningInvoice {
    pub id: Uuid,
    pub payment_hash: String,
    pub user_id: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub lightning_address: Option<String>,
    pub bolt11: String,
    pub network: String,
    pub description: Option<String>,
    pub description_hash: Option<String>,
    pub payee_pubkey: String,
    pub min_final_cltv_expiry_delta: u64,
    pub amount_msat: Option<u64>,
    pub payment_secret: String,
    pub timestamp: DateTime<Utc>,
    #[serde_as(as = "DurationSeconds<u64>")]
    pub expiry: Duration,
    pub status: LightningInvoiceStatus,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub fee_msat: Option<u64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub payment_time: Option<DateTime<Utc>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub label: Option<Uuid>,
    pub created_at: DateTime<Utc>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub updated_at: Option<DateTime<Utc>>,
    pub expires_at: DateTime<Utc>,
}

#[derive(Clone, Debug, EnumString, Deserialize, Serialize, Display, PartialEq, Eq, Default)]
pub enum LightningInvoiceStatus {
    #[default]
    PENDING,
    SETTLED,
    EXPIRED,
}

#[derive(Clone, Debug, Deserialize, Serialize, Default)]
pub struct LightningInvoiceFilter {
    pub user_id: Option<String>,
    pub status: Option<LightningInvoiceStatus>,
    pub limit: Option<u64>,
    pub offset: Option<u64>,
    pub id: Option<Uuid>,
}
