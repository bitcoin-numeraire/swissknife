//! Typed response models for assertions.
//!
//! These mirror the public API contract (the fields the tests assert on) and
//! are deserialized from responses instead of indexing into raw JSON. They are
//! intentionally partial — serde ignores fields we don't assert. They are kept
//! here rather than imported from the crate because the suite is a black box
//! over HTTP and the binary exposes no library; a shared `api-types` crate
//! (single source of truth for app + tests + clients) is the longer-term fix.

use serde::Deserialize;
use uuid::Uuid;

#[derive(Debug, Deserialize)]
pub struct AuthToken {
    pub token: String,
}

#[derive(Debug, Deserialize)]
pub struct Balance {
    pub available_msat: i64,
    pub reserved_msat: i64,
    pub received_msat: u64,
    pub sent_msat: u64,
    pub fees_paid_msat: u64,
}

#[derive(Debug, Deserialize)]
pub struct WalletResponse {
    pub id: Uuid,
    pub user_id: String,
    pub balance: Balance,
}

#[derive(Debug, Deserialize)]
pub struct LnInvoice {
    pub payment_hash: String,
    pub bolt11: String,
}

#[derive(Debug, Deserialize)]
pub struct InvoiceResponse {
    pub id: Uuid,
    pub status: String,
    pub amount_msat: Option<u64>,
    pub ln_invoice: Option<LnInvoice>,
}

#[derive(Debug, Deserialize)]
pub struct PaymentResponse {
    pub id: Uuid,
    pub status: String,
    pub amount_msat: u64,
    pub fee_msat: Option<u64>,
    pub ledger: String,
}

#[derive(Debug, Deserialize)]
pub struct BtcAddress {
    pub address: String,
}
