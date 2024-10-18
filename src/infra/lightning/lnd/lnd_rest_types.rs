use chrono::{TimeZone, Utc};
use lightning_invoice::Bolt11Invoice;
use serde::{Deserialize, Serialize};
use std::str::FromStr;

use crate::{
    application::entities::Ledger,
    domains::{
        invoice::{Invoice, InvoiceStatus},
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

#[derive(Debug, Deserialize)]
pub struct InvoiceResponse {
    pub payment_request: String,
    pub add_index: String,
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

#[derive(Debug, Serialize)]
pub struct ListInvoicesRequest {
    pub payment_hash: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct ListInvoicesResponse {
    pub invoices: Vec<ListInvoicesInvoice>,
}

#[derive(Debug, Serialize)]
pub struct GetinfoRequest {}

#[derive(Debug, Deserialize)]
pub struct GetinfoResponse {}

#[derive(Debug, Deserialize)]
pub struct ListInvoicesInvoice {
    bolt11: Option<String>,
    status: String,
    paid_at: Option<u64>,
    amount_received_msat: Option<u64>,
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

impl From<ListInvoicesInvoice> for Invoice {
    fn from(val: ListInvoicesInvoice) -> Self {
        let bolt11_str = val.bolt11.clone().unwrap();
        let bolt11 = Bolt11Invoice::from_str(&bolt11_str).unwrap();
        let mut invoice: Invoice = bolt11.into();

        match val.status.as_str() {
            "paid" => {
                invoice.status = InvoiceStatus::Settled;
                invoice.payment_time =
                    Some(Utc.timestamp_opt(val.paid_at.unwrap() as i64, 0).unwrap());
                invoice.amount_msat = val.amount_received_msat;
            }
            "unpaid" => {
                invoice.status = InvoiceStatus::Pending;
            }
            "expired" => {
                invoice.status = InvoiceStatus::Expired;
            }
            _ => {}
        };

        invoice
    }
}

#[derive(Debug, Deserialize)]
pub struct ErrorResponse {
    pub message: String,
}
