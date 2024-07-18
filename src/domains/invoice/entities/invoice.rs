use std::time::Duration;

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use serde_with::{serde_as, DisplayFromStr};
use strum_macros::{Display, EnumString};
use utoipa::{IntoParams, ToSchema};
use uuid::Uuid;

use crate::application::entities::{Currency, Ledger, OrderDirection};

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

#[derive(
    Clone, Debug, EnumString, Deserialize, Serialize, Display, PartialEq, Eq, Default, ToSchema,
)]
pub enum InvoiceStatus {
    #[default]
    Pending,
    Settled,
    Expired,
}

#[serde_as]
#[derive(Clone, Debug, Deserialize, Serialize, Default, IntoParams)]
pub struct InvoiceFilter {
    /// Total amount of results to return
    #[serde_as(as = "Option<DisplayFromStr>")]
    pub limit: Option<u64>,
    /// Offset where to start returning results
    #[serde_as(as = "Option<DisplayFromStr>")]
    pub offset: Option<u64>,
    /// List of IDs
    pub ids: Option<Vec<Uuid>>,
    /// Wallet ID. Automatically populated with your ID
    pub wallet_id: Option<Uuid>,
    /// Status
    pub status: Option<InvoiceStatus>,
    /// Ledger
    pub ledger: Option<Ledger>,
    /// Order by
    #[serde(default)]
    pub order_by: InvoiceOrderBy,
    /// Direction of the ordering of results
    #[serde(default)]
    pub order_direction: OrderDirection,
}

#[derive(Clone, Debug, Deserialize, Serialize, PartialEq, Eq, Default, ToSchema)]
pub enum InvoiceOrderBy {
    #[default]
    CreatedAt,
    PaymentTime,
    UpdatedAt,
}
