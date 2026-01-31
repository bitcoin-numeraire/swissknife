use base64::{prelude::BASE64_STANDARD, Engine};
use chrono::{TimeZone, Utc};
use lightning_invoice::Bolt11Invoice;
use serde::{Deserialize, Serialize};

use crate::domains::{
    bitcoin::{BtcTransaction, BtcTransactionOutput},
    event::LnInvoicePaidEvent,
    payment::LnPayment,
};
use serde_with::{serde_as, DisplayFromStr};
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

#[derive(Debug, Serialize)]
pub struct TrackPaymentRequest {
    pub no_inflight_updates: bool,
}

#[serde_as]
#[derive(Debug, Deserialize)]
pub struct TrackPaymentResponse {
    pub payment_hash: String,
    pub payment_preimage: String,
    #[serde_as(as = "Option<DisplayFromStr>")]
    pub value_msat: Option<u64>,
    #[serde_as(as = "Option<DisplayFromStr>")]
    pub fee_msat: Option<u64>,
    #[serde_as(as = "Option<DisplayFromStr>")]
    pub creation_time_ns: Option<i64>,
    #[serde_as(as = "Option<DisplayFromStr>")]
    pub payment_time_ns: Option<i64>,
    pub status: String,
    #[serde(default)]
    pub failure_reason: String,
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
            amount_msat: val.value_msat,
            fee_msat: Some(val.fee_msat),
            payment_time: Some(Utc.timestamp_nanos(val.creation_time_ns)),
            lightning: Some(LnPayment {
                payment_hash: Some(val.payment_hash),
                payment_preimage: Some(val.payment_preimage),
                ..Default::default()
            }),
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

#[serde_as]
#[derive(Debug, Deserialize)]
pub struct TransactionResponse {
    pub tx_hash: String,
    pub block_height: u32,
    #[serde_as(as = "DisplayFromStr")]
    pub time_stamp: i64,
    #[serde_as(as = "DisplayFromStr")]
    pub total_fees: u64,
    pub output_details: Vec<OutputDetailResponse>,
}

#[serde_as]
#[derive(Debug, Deserialize)]
pub struct OutputDetailResponse {
    #[serde_as(as = "DisplayFromStr")]
    pub output_index: u32,
    #[serde_as(as = "DisplayFromStr")]
    pub amount: u64,
    pub address: String,
    pub is_our_address: bool,
}

impl From<TransactionResponse> for BtcTransaction {
    fn from(val: TransactionResponse) -> Self {
        let outputs = val
            .output_details
            .into_iter()
            .map(|detail| BtcTransactionOutput {
                output_index: detail.output_index,
                address: Some(detail.address),
                amount_sat: detail.amount,
                is_ours: detail.is_our_address,
            })
            .collect();

        BtcTransaction {
            txid: val.tx_hash,
            timestamp: Some(Utc.timestamp_opt(val.time_stamp, 0).unwrap()),
            fee_sat: Some(val.total_fees),
            block_height: Some(val.block_height),
            outputs,
        }
    }
}
