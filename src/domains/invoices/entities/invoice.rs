use std::time::Duration;

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use serde_with::{serde_as, DisplayFromStr, DurationSeconds};
use strum_macros::{Display, EnumString};
use utoipa::{IntoParams, ToSchema};
use uuid::Uuid;

use crate::application::entities::OrderDirection;
use crate::application::entities::{Currency, Ledger};

#[serde_as]
#[derive(Clone, Debug, Default, Serialize, ToSchema)]
pub struct Invoice {
    /// Internal ID
    pub id: Uuid,
    /// User ID
    pub user_id: String,
    /// Lightning Address. Populated when invoice is generated as part of the LNURL protocol
    #[serde(skip_serializing_if = "Option::is_none")]
    pub ln_address_id: Option<Uuid>,
    /// Description
    pub description: Option<String>,
    /// Currency. Different networks use different currencies such as testnet
    pub currency: Currency,
    /// Amount in millisatoshis.
    pub amount_msat: Option<u64>,
    /// Date of creation on the LN node
    pub timestamp: DateTime<Utc>,
    /// Status
    pub status: InvoiceStatus,
    /// Ledger
    pub ledger: Ledger,
    #[serde(skip_serializing_if = "Option::is_none")]
    /// Fee paid. Populated when a new channel is opened to receive the funds.
    pub fee_msat: Option<u64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    /// Payment time
    pub payment_time: Option<DateTime<Utc>>,
    /// Date of creation in database
    pub created_at: DateTime<Utc>,
    #[serde(skip_serializing_if = "Option::is_none")]
    /// Date of update in database
    pub updated_at: Option<DateTime<Utc>>,

    #[serde(flatten)]
    #[serde(skip_serializing_if = "Option::is_none")]
    /// Lightning details of the invoice
    pub lightning: Option<LnInvoice>,
}

#[serde_as]
#[derive(Clone, Debug, Default, Serialize, ToSchema)]
pub struct LnInvoice {
    /// Payment hash
    #[schema(example = "b587c7f76339e3fb87ad2b...")]
    pub payment_hash: String,

    /// Bolt11
    #[schema(example = "lnbcrt1m1png24kasp5...")]
    pub bolt11: String,

    /// Description hash
    #[serde(skip_serializing_if = "Option::is_none")]
    pub description_hash: Option<String>,

    /// Public key of the node receiving the funds
    #[schema(example = "02086a3f5b67ac4c43...")]
    pub payee_pubkey: String,

    /// The minimum number of blocks the final hop in the route should wait before allowing the payment to be claimed. This is a security measure to ensure that the payment can be settled properly
    #[schema(example = 10)]
    pub min_final_cltv_expiry_delta: u64,

    /// A secret value included in the payment request to mitigate certain types of attacks. The payment secret must be provided by the payer when making the payment
    #[schema(example = "019a32e03bb375a42bc...")]
    pub payment_secret: String,

    /// Duration of expiry in seconds since creation
    #[serde_as(as = "DurationSeconds<u64>")]
    #[schema(example = 3600)]
    pub expiry: Duration,

    /// Date of expiry
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
    /// User ID. Automatically populated with your user ID
    pub user_id: Option<String>,
    /// Status
    pub status: Option<InvoiceStatus>,
    /// Ledger
    pub ledger: Option<Ledger>,
    #[serde(default)]
    /// Direction of the ordering of results
    pub order_direction: OrderDirection,
}
