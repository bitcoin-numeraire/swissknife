use breez_sdk_core::PaymentDetails;
use chrono::{DateTime, Utc};
use serde::Serialize;
use uuid::Uuid;

#[derive(Clone, Debug, Serialize)]
pub struct LightningPayment {
    pub id: Option<Uuid>,
    pub lightning_address: Option<String>,
    pub payment_hash: String,
    pub error: Option<String>,
    pub amount_msat: i64,
    pub fee_msat: Option<i64>,
    pub payment_time: Option<i64>,
    pub status: String, // TODO: Use enum
    pub description: Option<String>,
    pub metadata: Option<String>,
    pub details: Option<PaymentDetails>, // Could be any type, currently only Breez data
    pub created_at: Option<DateTime<Utc>>,
    pub updated_at: Option<DateTime<Utc>>,
}

impl LightningPayment {
    pub fn new(payment_hash: String, amount_msat: i64, error: Option<String>) -> Self {
        Self {
            id: None,
            lightning_address: None,
            payment_hash,
            error: error.clone(),
            amount_msat,
            fee_msat: None,
            payment_time: None,
            status: if error.is_some() {
                "FAILED".to_string()
            } else {
                "PENDING".to_string()
            },
            description: None,
            metadata: None,
            details: None,
            created_at: None,
            updated_at: None,
        }
    }
}
