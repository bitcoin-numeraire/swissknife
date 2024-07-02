use chrono::{TimeZone, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::{application::entities::Ledger, domains::payments::entities::Payment};

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
}

#[derive(Debug, Serialize)]
pub struct PayRequest {
    pub bolt11: String,
    pub label: Option<String>,
    pub maxfeepercent: Option<f64>,
    pub retry_for: Option<u32>,
    pub exemptfee: Option<u64>,
    pub amount_msat: Option<u64>,
}

#[derive(Debug, Deserialize)]
pub struct PayResponse {
    pub payment_preimage: String,
    pub payment_hash: String,
    pub created_at: f64,
    pub amount_msat: u64,
    pub amount_sent_msat: u64,
    pub status: String,
}

impl From<PayResponse> for Payment {
    fn from(val: PayResponse) -> Self {
        let error = match val.status.as_str() {
            "complete" => None,
            _ => Some(format!(
                "Unexpected error. Payment returned successfully but with status {}",
                val.status
            )),
        };

        let seconds = val.created_at as i64;
        let nanoseconds = ((val.created_at - seconds as f64) * 1e9) as u32;

        Payment {
            ledger: Ledger::Lightning,
            payment_hash: Some(val.payment_hash),
            payment_preimage: Some(val.payment_preimage),
            amount_msat: val.amount_sent_msat,
            fee_msat: Some(val.amount_sent_msat - val.amount_msat),
            payment_time: Some(Utc.timestamp_opt(seconds, nanoseconds).unwrap()),
            error,
            ..Default::default()
        }
    }
}

#[derive(Debug, Deserialize)]
pub struct ErrorResponse {
    pub message: String,
}
