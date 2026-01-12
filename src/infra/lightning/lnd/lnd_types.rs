use base64::{prelude::BASE64_STANDARD, Engine};
use chrono::{TimeZone, Utc};
use lightning_invoice::Bolt11Invoice;
use serde::{Deserialize, Serialize};

use crate::domains::bitcoin::{BitcoinTransaction, BitcoinTransactionOutput};
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
    pub payment_request: String,
    pub fee_limit_msat: u64,
    pub amt_msat: Option<u64>,
    pub timeout_seconds: u32,
    pub no_inflight_updates: bool,
}

#[derive(Deserialize)]
pub struct StreamPayResponse {
    pub result: Option<PayResponse>,
    pub error: Option<ErrorResponse>,
}

#[serde_as]
#[derive(Debug, Deserialize)]
pub struct PayResponse {
    pub payment_preimage: String,
    pub payment_hash: String,
    #[serde_as(as = "DisplayFromStr")]
    pub value_msat: u64,
    #[serde_as(as = "DisplayFromStr")]
    pub creation_time_ns: i64,
    #[serde_as(as = "DisplayFromStr")]
    pub fee_msat: u64,
    pub status: String,
    pub failure_reason: String,
}

#[derive(Debug, Deserialize)]
pub struct GetinfoResponse {
    pub chains: Option<Vec<Chain>>,
}

#[derive(Debug, Deserialize)]
pub struct Chain {
    pub network: Option<String>,
}

impl From<PayResponse> for Payment {
    fn from(val: PayResponse) -> Self {
        Payment {
            ledger: Ledger::Lightning,
            payment_hash: Some(val.payment_hash),
            payment_preimage: Some(val.payment_preimage),
            amount_msat: val.value_msat,
            fee_msat: Some(val.fee_msat),
            payment_time: Some(Utc.timestamp_nanos(val.creation_time_ns)),
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
            payment_hash: hex_from_base64(&val.r_hash),
            amount_received_msat: val.amt_paid_msat,
            fee_msat: 0,
            payment_time: Utc.timestamp_opt(val.settle_date, 0).unwrap(),
        }
    }
}

fn hex_from_base64(s: &str) -> String {
    hex::encode(BASE64_STANDARD.decode(s).expect("should be valid base64"))
}

#[derive(Debug, Deserialize)]
pub struct ErrorResponse {
    pub message: String,
}

#[derive(Debug, Serialize)]
pub struct SendCoinsRequest {
    pub addr: String,
    pub amount: i64,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub sat_per_vbyte: Option<u64>,
}

#[derive(Debug, Deserialize)]
pub struct SendCoinsResponse {
    pub txid: String,
}

#[derive(Debug, Deserialize)]
pub struct NewAddressResponse {
    pub address: String,
}

#[derive(Debug, Deserialize)]
pub struct Transaction {
    pub tx_hash: Option<String>,
    pub num_confirmations: Option<u32>,
    pub block_height: Option<u32>,
    pub time_stamp: Option<String>,
    pub total_fees: Option<String>,
    pub output_details: Option<Vec<OutputDetail>>,
}

#[derive(Debug, Deserialize)]
pub struct OutputDetail {
    pub output_index: Option<u32>,
    pub amount: Option<String>,
    pub address: Option<String>,
    pub is_ours: Option<bool>,
}

fn parse_amount(value: Option<String>) -> u64 {
    value.and_then(|v| v.parse::<u64>().ok()).unwrap_or_default()
}

impl From<Transaction> for BitcoinTransaction {
    fn from(val: Transaction) -> Self {
        let timestamp = val.time_stamp.and_then(|value| value.parse::<i64>().ok());

        let outputs = val
            .output_details
            .unwrap_or_default()
            .into_iter()
            .filter_map(|detail| {
                Some(BitcoinTransactionOutput {
                    output_index: detail.output_index?,
                    address: detail.address,
                    amount_sat: parse_amount(detail.amount),
                    is_ours: detail.is_ours.unwrap_or_default(),
                })
            })
            .collect();

        BitcoinTransaction {
            txid: val.tx_hash.unwrap_or_default(),
            timestamp: timestamp.and_then(|t| chrono::Utc.timestamp_opt(t, 0).single()),
            fee_sat: val.total_fees.map(|fees| parse_amount(Some(fees))),
            block_height: val.block_height,
            confirmations: val.num_confirmations,
            outputs,
        }
    }
}
