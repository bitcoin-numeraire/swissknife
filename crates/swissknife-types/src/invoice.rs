use std::time::Duration;

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use serde_with::{serde_as, DurationSeconds};
use strum_macros::{Display, EnumString};
use utoipa::ToSchema;
use uuid::Uuid;

use crate::{BtcOutput, Currency, Ledger};

#[derive(Clone, Debug, Default, Serialize, ToSchema)]
pub struct Invoice {
    /// Internal ID
    pub id: Uuid,
    /// Wallet ID
    pub wallet_id: Uuid,

    /// Lightning Address. Populated when invoice is generated as part of the LNURL protocol
    #[serde(skip_serializing_if = "Option::is_none")]
    pub ln_address_id: Option<Uuid>,

    /// Description
    pub description: Option<String>,
    /// Amount requested in millisatoshis.
    pub amount_msat: Option<u64>,
    /// Amount received in millisatoshis.
    pub amount_received_msat: Option<u64>,
    /// Date of creation on the LN node
    pub timestamp: DateTime<Utc>,
    /// Status
    pub status: InvoiceStatus,
    /// Ledger
    pub ledger: Ledger,
    /// Currency
    pub currency: Currency,

    /// Fees paid. Populated when a new channel is opened to receive the funds.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub fee_msat: Option<u64>,

    /// Payment time
    #[serde(skip_serializing_if = "Option::is_none")]
    pub payment_time: Option<DateTime<Utc>>,

    /// Date of creation in database
    pub created_at: DateTime<Utc>,

    /// Date of update in database
    #[serde(skip_serializing_if = "Option::is_none")]
    pub updated_at: Option<DateTime<Utc>>,

    /// Lightning details of the invoice
    #[serde(skip_serializing_if = "Option::is_none")]
    pub ln_invoice: Option<LnInvoice>,

    /// Internal reference to the on-chain output backing this invoice.
    #[serde(skip)]
    pub btc_output_id: Option<Uuid>,

    /// Bitcoin output details of the invoice
    #[serde(skip_serializing_if = "Option::is_none")]
    pub bitcoin_output: Option<BtcOutput>,
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
    #[schema(value_type = u64, example = 3600)]
    pub expiry: Duration,

    /// Date of expiry
    pub expires_at: DateTime<Utc>,
}

#[derive(Clone, Debug, EnumString, Deserialize, Serialize, Display, PartialEq, Eq, Default, ToSchema)]
pub enum InvoiceStatus {
    #[default]
    Pending,
    Settled,
    Expired,
}
