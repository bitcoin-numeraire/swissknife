use serde::{Deserialize, Serialize};
use uuid::Uuid;

#[derive(Debug, Serialize)]
pub struct InvoiceRequest {
    pub description: String,
    pub label: Uuid,
    pub expiry: u64,
    pub amount_msat: u64,
}

#[derive(Debug, Deserialize)]
pub struct InvoiceResponse {
    pub bolt11: String,
    pub payment_hash: String,
    pub payment_secret: String,
    pub expires_at: u64,
    pub warning_capacity: Option<String>,
    pub warning_offline: Option<String>,
    pub warning_deadends: Option<String>,
    pub warning_private_unused: Option<String>,
    pub warning_mpp: Option<String>,
    pub created_index: Option<u64>,
}
