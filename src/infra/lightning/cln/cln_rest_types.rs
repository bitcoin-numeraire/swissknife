use chrono::{TimeZone, Utc};
use lightning_invoice::Bolt11Invoice;
use serde::{Deserialize, Serialize};
use std::str::FromStr;
use uuid::Uuid;

use crate::{
    application::entities::Ledger,
    domains::{
        invoice::{Invoice, InvoiceStatus},
        payment::{LnPayment, Payment},
    },
};

#[derive(Debug, Serialize)]
pub struct InvoiceRequest {
    pub description: String,
    pub label: Uuid,
    pub expiry: u64,
    pub amount_msat: u64,
    pub deschashonly: Option<bool>,
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

#[derive(Debug, Serialize)]
pub struct ListInvoicesRequest {
    pub payment_hash: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct ListInvoicesResponse {
    pub invoices: Vec<ListInvoicesInvoice>,
}

#[derive(Debug, Serialize, Default)]
pub struct ListTransactionsRequest {}

#[derive(Debug, Deserialize)]
pub struct ListTransactionsResponse {
    pub transactions: Vec<ListTransactionsTransaction>,
}

#[derive(Debug, Deserialize)]
pub struct ListTransactionsTransaction {
    pub hash: String,
    pub blockheight: u32,
    pub outputs: Vec<ListTransactionsOutput>,
}

#[derive(Debug, Deserialize)]
pub struct ListTransactionsOutput {
    pub index: u32,
    pub amount_msat: u64,

    #[serde(rename = "scriptPubKey")]
    pub script_pub_key: String,
}

#[derive(Debug, Serialize, Default)]
pub struct ListChainMovesRequest {
    pub index: Option<String>,
    pub start: Option<u64>,
    pub limit: Option<u64>,
}

#[derive(Debug, Deserialize)]
pub struct ListChainMovesResponse {
    pub chainmoves: Vec<ListChainMove>,
}

#[derive(Debug, Deserialize)]
pub struct ListChainMove {
    pub created_index: u64,
    pub primary_tag: String,
    pub account_id: String,
    pub utxo: String,
    pub spending_txid: Option<String>,
    pub blockheight: Option<u32>,
}

#[derive(Debug, Serialize, Default)]
pub struct ListPaysRequest {
    pub payment_hash: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct ListPaysResponse {
    pub pays: Vec<ListPaysPayment>,
}

#[derive(Debug, Deserialize)]
pub struct ListPaysPayment {
    pub status: String,
    pub payment_hash: String,
    pub preimage: Option<String>,
    pub amount_msat: Option<u64>,
    pub amount_sent_msat: Option<u64>,
    pub created_at: Option<u64>,
    pub completed_at: Option<u64>,
}

#[derive(Debug, Serialize, Default)]
pub struct ListFundsRequest {
    pub spent: Option<bool>,
}

#[derive(Debug, Deserialize)]
pub struct ListFundsResponse {
    pub outputs: Vec<ListFundsOutput>,
}

#[derive(Debug, Deserialize)]
pub struct ListFundsOutput {
    pub txid: String,
    pub output: u32,
    pub amount_msat: u64,
    pub address: Option<String>,
    pub status: String,
    pub blockheight: Option<u32>,
}

#[derive(Debug, Serialize)]
pub struct GetinfoRequest {}

#[derive(Debug, Deserialize)]
pub struct GetinfoResponse {
    pub network: String,
}

#[derive(Debug, Serialize)]
pub struct NewAddrRequest {
    pub addresstype: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct NewAddrResponse {
    pub bech32: Option<String>,
    #[serde(rename = "p2tr")]
    pub p2tr: Option<String>,
}

#[derive(Debug, Serialize)]
pub struct TxPrepareRequest {
    pub outputs: Vec<TxPrepareOutput>,
    pub feerate: Option<u32>,
}

#[derive(Debug)]
pub struct TxPrepareOutput {
    pub address: String,
    pub amount: u64,
}

impl Serialize for TxPrepareOutput {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        use serde::ser::SerializeMap;
        let mut map = serializer.serialize_map(Some(1))?;
        map.serialize_entry(&self.address, &self.amount)?;
        map.end()
    }
}

#[derive(Debug, Deserialize)]
pub struct TxPrepareResponse {
    pub psbt: String,
    pub txid: String,
}

#[derive(Debug, Serialize)]
pub struct TxSendRequest {
    pub txid: String,
}

#[derive(Debug, Deserialize)]
pub struct TxSendResponse {
    #[allow(dead_code)]
    pub txid: String,
}

#[derive(Debug, Serialize)]
pub struct TxDiscardRequest {
    pub txid: String,
}

#[derive(Debug, Deserialize)]
pub struct TxDiscardResponse {
    #[allow(dead_code)]
    pub txid: String,
}

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
            amount_msat: val.amount_sent_msat,
            fee_msat: Some(val.amount_sent_msat - val.amount_msat),
            payment_time: Some(Utc.timestamp_opt(seconds, nanoseconds).unwrap()),
            error,
            lightning: Some(LnPayment {
                payment_hash: val.payment_hash,
                payment_preimage: Some(val.payment_preimage),
                ..Default::default()
            }),
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
                invoice.payment_time = Some(Utc.timestamp_opt(val.paid_at.unwrap() as i64, 0).unwrap());
                invoice.amount_received_msat = val.amount_received_msat;
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
