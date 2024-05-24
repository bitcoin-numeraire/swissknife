use chrono::{DateTime, FixedOffset};
use strum_macros::{Display, EnumString};
use uuid::Uuid;

#[derive(Clone, Debug, Default)]
pub struct LightningInvoice {
    pub id: Uuid,
    pub payment_hash: String,
    pub user_id: String,
    pub lightning_address: Option<String>,
    pub bolt11: String,
    pub network: String,
    pub description: Option<String>,
    pub description_hash: Option<String>,
    pub payee_pubkey: String,
    pub min_final_cltv_expiry_delta: u64,
    pub amount_msat: Option<u64>,
    pub payment_secret: Vec<u8>,
    pub timestamp: u64,
    pub expiry: u64,
    pub status: LightningInvoiceStatus,
    pub fee_msat: Option<u64>,
    pub payment_time: Option<i64>,
    pub label: Option<Uuid>,
    pub created_at: DateTime<FixedOffset>,
    pub updated_at: Option<DateTime<FixedOffset>>,
}

#[derive(Clone, Debug, EnumString, Display, PartialEq, Eq, Default)]
pub enum LightningInvoiceStatus {
    #[default]
    PENDING,
    SETTLED,
    EXPIRED,
}

pub struct LightningInvoiceDeleteFilter {
    pub expired: Option<bool>,
}
