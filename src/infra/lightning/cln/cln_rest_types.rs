use chrono::{TimeZone, Utc};
use lightning_invoice::Bolt11Invoice;
use serde::{Deserialize, Serialize};
use serde_bolt::bitcoin::hashes::{sha256, Hash};
use std::str::FromStr;

use crate::{
    application::composition::Ledger,
    domains::{
        invoice::{Invoice, InvoiceStatus},
        payment::{LnPayment, Payment},
    },
};

#[derive(Debug, Serialize)]
pub struct InvoiceRequest {
    pub description: String,
    pub label: String,
    pub expiry: u64,
    pub amount_msat: u64,
    pub deschashonly: Option<bool>,
}

#[derive(Debug, Deserialize)]
pub struct InvoiceResponse {
    pub bolt11: String,
}

#[derive(Debug, Serialize)]
pub struct XpayRequest {
    pub invstring: String,
    pub amount_msat: Option<u64>,
    pub maxfee: Option<u64>,
    pub retry_for: Option<u32>,
    pub label: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct XpayResponse {
    pub payment_preimage: String,
    pub amount_msat: u64,
    pub amount_sent_msat: u64,
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
pub struct DelInvoiceRequest {
    pub label: String,
    pub status: String,
    pub desconly: Option<bool>,
}

#[derive(Debug, Deserialize)]
pub struct DelInvoiceResponse {}

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
    pub id: String,
    pub network: String,
}

#[derive(Debug, Serialize)]
pub struct GetRoutesRequest {
    pub source: String,
    pub destination: String,
    pub amount_msat: u64,
    pub layers: Vec<String>,
    pub maxfee_msat: u64,
    pub final_cltv: u32,
    pub maxparts: u32,
}

#[derive(Debug, Deserialize)]
pub struct GetRoutesResponse {
    pub routes: Vec<GetRoutesRoute>,
}

#[derive(Debug, Deserialize)]
pub struct GetRoutesRoute {
    pub amount_msat: u64,
    pub path: Vec<GetRoutesPath>,
}

#[derive(Debug, Deserialize)]
pub struct GetRoutesPath {
    pub amount_in_msat: Option<u64>,
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
pub struct SetPsbtVersionRequest {
    pub psbt: String,
    pub version: u32,
}

#[derive(Debug, Deserialize)]
pub struct SetPsbtVersionResponse {
    pub psbt: String,
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

impl From<XpayResponse> for Payment {
    fn from(val: XpayResponse) -> Self {
        // `xpay` returns no payment_hash; it is the SHA-256 of the preimage.
        let preimage_bytes = hex::decode(&val.payment_preimage).unwrap_or_default();
        let payment_hash = hex::encode(sha256::Hash::hash(&preimage_bytes).to_byte_array());

        Payment {
            ledger: Ledger::Lightning,
            // `amount_msat` is the amount delivered to the recipient; `amount_sent_msat`
            // includes the routing fee. Settlement debits `amount_msat + fee_msat`, so
            // storing the delivered amount (not the fee-inclusive total) avoids charging
            // the fee twice. Matches the gRPC converter.
            amount_msat: val.amount_msat,
            fee_msat: Some(val.amount_sent_msat.saturating_sub(val.amount_msat)),
            // A returned XpayResponse means the payment completed; xpay surfaces
            // failures as an error response. No created_at is returned, so stamp now.
            payment_time: Some(Utc::now()),
            error: None,
            lightning: Some(LnPayment {
                payment_hash,
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
        let mut invoice: Invoice = crate::infra::lightning::types::invoice_from_bolt11(bolt11);

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

#[cfg(test)]
mod tests {
    use super::*;

    /// Guards against debiting the routing fee twice. The settlement path debits
    /// `amount_msat + fee_msat`, so `amount_msat` must be the delivered amount, not
    /// the fee-inclusive `amount_sent_msat`. The integration suite cannot catch this
    /// because its single-hop regtest topology always has a zero routing fee.
    #[test]
    fn xpay_response_separates_delivered_amount_from_routing_fee() {
        let resp = XpayResponse {
            payment_preimage: "01".repeat(32),
            amount_msat: 100_000,      // delivered to the recipient
            amount_sent_msat: 100_500, // delivered + 500 msat routing fee
        };

        let payment: Payment = resp.into();

        assert_eq!(payment.amount_msat, 100_000);
        assert_eq!(payment.fee_msat, Some(500));
        assert_eq!(
            payment.amount_msat + payment.fee_msat.unwrap(),
            100_500,
            "wallet debit (amount + fee) must equal amount_sent_msat, not amount + 2*fee"
        );
    }
}
