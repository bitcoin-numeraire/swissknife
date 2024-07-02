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

impl Into<Payment> for PayResponse {
    fn into(self) -> Payment {
        let error = match self.status.as_str() {
            "complete" => None,
            _ => Some(format!(
                "Unexpected error. Payment returned successfully but with status {}",
                self.status
            )),
        };

        let seconds = self.created_at as i64;
        let nanoseconds = ((self.created_at - seconds as f64) * 1e9) as u32;

        Payment {
            ledger: Ledger::LIGHTNING,
            payment_hash: Some(self.payment_hash),
            payment_preimage: Some(self.payment_preimage),
            amount_msat: self.amount_sent_msat,
            fee_msat: Some(self.amount_sent_msat - self.amount_msat),
            payment_time: Some(Utc.timestamp_opt(seconds, nanoseconds).unwrap()),
            error,
            ..Default::default()
        }
    }
}
