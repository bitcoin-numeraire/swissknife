use chrono::{DateTime, FixedOffset};
use serde::Serialize;
use uuid::Uuid;

#[derive(Clone, Debug, Serialize, Default)]
pub struct LightningInvoice {
    pub id: Uuid,
    pub lightning_address: Option<String>,
    pub bolt11: String,
    pub network: String,
    pub payee_pubkey: String,
    pub payment_hash: String,
    pub description: Option<String>,
    pub description_hash: Option<String>,
    pub amount_msat: Option<u64>,
    pub payment_secret: Vec<u8>,
    pub timestamp: u64,
    pub expiry: u64,
    pub min_final_cltv_expiry_delta: u64,
    pub status: String,
    pub fee_msat: Option<u64>,
    pub payment_time: Option<i64>,
    pub created_at: DateTime<FixedOffset>,
    pub updated_at: Option<DateTime<FixedOffset>>,
}
