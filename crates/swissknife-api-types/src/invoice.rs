use std::time::Duration;

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use strum_macros::{Display, EnumString};
use utoipa::ToSchema;
use uuid::Uuid;

use crate::{BtcOutput, Currency, Ledger};

#[derive(Clone, Debug, Default)]
pub struct Invoice {
    pub id: Uuid,
    pub wallet_id: Uuid,
    pub ln_address_id: Option<Uuid>,
    pub description: Option<String>,
    pub amount_msat: Option<u64>,
    pub amount_received_msat: Option<u64>,
    pub timestamp: DateTime<Utc>,
    pub status: InvoiceStatus,
    pub ledger: Ledger,
    pub currency: Currency,
    pub fee_msat: Option<u64>,
    pub payment_time: Option<DateTime<Utc>>,
    pub created_at: DateTime<Utc>,
    pub updated_at: Option<DateTime<Utc>>,
    pub ln_invoice: Option<LnInvoice>,
    pub btc_output_id: Option<Uuid>,
    pub bitcoin_output: Option<BtcOutput>,
}

#[derive(Clone, Debug, Default)]
pub struct LnInvoice {
    pub payment_hash: String,
    pub bolt11: String,
    pub description_hash: Option<String>,
    pub payee_pubkey: String,
    pub min_final_cltv_expiry_delta: u64,
    pub payment_secret: String,
    pub expiry: Duration,
    pub expires_at: DateTime<Utc>,
}

#[derive(Clone, Debug, EnumString, Deserialize, Serialize, Display, PartialEq, Eq, Default, ToSchema)]
pub enum InvoiceStatus {
    #[default]
    Pending,
    Settled,
    Expired,
}
