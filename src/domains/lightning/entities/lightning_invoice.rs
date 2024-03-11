use chrono::{DateTime, Utc};
use serde::Serialize;
use uuid::Uuid;

#[derive(Clone, Debug, Serialize)]
pub struct LightningInvoice {
    pub id: Option<Uuid>,
    pub lightning_address: Option<String>,
    pub bolt11: String,
    pub network: String,
    pub payee_pubkey: String,
    pub payment_hash: String,
    pub description: Option<String>,
    pub comment: Option<String>,
    pub description_hash: Option<String>,
    pub amount_msat: Option<i64>,
    pub payment_secret: Vec<u8>,
    pub timestamp: i64,
    pub expiry: i64,
    pub min_final_cltv_expiry_delta: i64,
    pub status: String,
    pub fee_msat: Option<i64>,
    pub payment_time: Option<i64>,
    pub created_at: Option<DateTime<Utc>>,
    pub updated_at: Option<DateTime<Utc>>,
}
