use base64::{prelude::BASE64_STANDARD, Engine};
use chrono::{TimeZone, Utc};
use lightning_invoice::Bolt11Invoice;
use serde::{Deserialize, Serialize};
use serde_with::{serde_as, DisplayFromStr};
use std::str::FromStr;

use crate::{
    application::entities::Ledger,
    domains::{
        invoice::{Invoice, InvoiceStatus},
        ln_node::LnInvoicePaidEvent,
        payment::Payment,
    },
};

#[derive(Debug, Serialize, Default)]
pub struct InvoiceRequest {
    pub memo: String,
    pub expiry: u64,
    pub value_msat: u64,
    pub description_hash: String,
}

#[serde_as]
#[derive(Debug, Deserialize)]
pub struct AddInvoiceResponse {
    pub payment_request: String,
}

#[serde_as]
#[derive(Debug, Deserialize)]
pub struct InvoiceResponse {
    pub payment_request: String,
    pub r_hash: String,
    pub state: String,
    #[serde_as(as = "DisplayFromStr")]
    pub amt_paid_msat: u64,
    #[serde_as(as = "DisplayFromStr")]
    pub settle_date: i64,
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

#[derive(Debug, Deserialize)]
pub struct GetinfoResponse {}

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

impl From<AddInvoiceResponse> for Invoice {
    fn from(val: AddInvoiceResponse) -> Self {
        Bolt11Invoice::from_str(&val.payment_request)
            .expect("should be valid BOLT11")
            .into()
    }
}

impl From<InvoiceResponse> for Invoice {
    fn from(val: InvoiceResponse) -> Self {
        let bolt11 = Bolt11Invoice::from_str(&val.payment_request).expect("should be valid BOLT11");
        let mut invoice: Invoice = bolt11.into();

        match val.state.as_str() {
            "SETTLED" => {
                invoice.status = InvoiceStatus::Settled;
                invoice.payment_time = Some(Utc.timestamp_opt(val.settle_date, 0).unwrap());
                invoice.amount_received_msat = Some(val.amt_paid_msat);
            }
            "OPEN" | "ACCEPTED" => {
                invoice.status = InvoiceStatus::Pending;
            }
            "CANCELED" => {
                invoice.status = InvoiceStatus::Expired;
            }
            _ => {}
        };

        invoice
    }
}

impl From<InvoiceResponse> for LnInvoicePaidEvent {
    fn from(val: InvoiceResponse) -> Self {
        LnInvoicePaidEvent {
            id: None,
            payment_hash: Some(hex::encode(
                BASE64_STANDARD
                    .decode(val.r_hash)
                    .expect("should be valid base64"),
            )),
            amount_received_msat: val.amt_paid_msat,
            fee_msat: 0,
            payment_time: Utc.timestamp_opt(val.settle_date, 0).unwrap(),
        }
    }
}

#[derive(Debug, Deserialize)]
pub struct ErrorResponse {
    pub message: String,
}
