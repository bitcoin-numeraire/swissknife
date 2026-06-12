use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use strum_macros::{Display, EnumString};
use utoipa::ToSchema;
use uuid::Uuid;

use crate::{Currency, Ledger, LnUrlSuccessAction};

#[derive(Clone, Debug, Default, Serialize, ToSchema)]
pub struct Payment {
    /// Internal ID
    pub id: Uuid,

    /// Wallet ID
    pub wallet_id: Uuid,

    /// Error message
    #[serde(skip_serializing_if = "Option::is_none")]
    #[schema(example = "failed to pay error message")]
    pub error: Option<String>,

    /// Amount in millisatoshis.
    pub amount_msat: u64,

    /// Fees paid. Populated when a new channel is opened to receive the funds
    #[serde(skip_serializing_if = "Option::is_none")]
    pub fee_msat: Option<u64>,

    /// Amount reserved internally for this pending outgoing payment.
    #[serde(skip)]
    pub reserved_amount: u64,

    /// Ledger
    pub ledger: Ledger,

    /// Currency
    pub currency: Currency,

    /// Payment time
    #[serde(skip_serializing_if = "Option::is_none")]
    pub payment_time: Option<DateTime<Utc>>,

    /// Status
    pub status: PaymentStatus,

    /// Description
    #[serde(skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,

    /// Date of creation in database
    pub created_at: DateTime<Utc>,

    /// Date of update in database
    #[serde(skip_serializing_if = "Option::is_none")]
    pub updated_at: Option<DateTime<Utc>>,

    /// Lightning payment details
    #[serde(skip_serializing_if = "Option::is_none")]
    pub lightning: Option<LnPayment>,

    /// Bitcoin on-chain payment details
    #[serde(skip_serializing_if = "Option::is_none")]
    pub bitcoin: Option<BtcPayment>,

    /// Internal payment details
    #[serde(skip_serializing_if = "Option::is_none")]
    pub internal: Option<InternalPayment>,
}

#[derive(Clone, Debug, Default, Serialize, ToSchema)]
pub struct LnPayment {
    /// Lightning Address. Populated when sending to a LN Address
    #[serde(skip_serializing_if = "Option::is_none")]
    #[schema(example = "hello@numeraire.tech")]
    pub ln_address: Option<String>,

    /// Payment hash
    #[schema(example = "b587c7f76339e3fb87ad2b...")]
    pub payment_hash: String,

    /// Payment Preimage
    #[serde(skip_serializing_if = "Option::is_none")]
    pub payment_preimage: Option<String>,

    /// Metadata
    #[serde(skip_serializing_if = "Option::is_none")]
    pub metadata: Option<String>,

    /// Success Action. Populated when sending to a LNURL or LN Address
    #[serde(skip_serializing_if = "Option::is_none")]
    pub success_action: Option<LnUrlSuccessAction>,
}

#[derive(Clone, Debug, Default, Serialize, ToSchema)]
pub struct BtcPayment {
    /// Destination Bitcoin address. Populated for Bitcoin onchain payments.
    pub address: String,

    /// Transaction ID for on-chain payments.
    pub txid: String,

    /// Bitcoin block height where the transaction was confirmed.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub block_height: Option<u32>,
}

#[derive(Clone, Debug, Default, Serialize, ToSchema)]
pub struct InternalPayment {
    /// Lightning Address. Populated for internal LN Address payments
    #[serde(skip_serializing_if = "Option::is_none")]
    #[schema(example = "hello@numeraire.tech")]
    pub ln_address: Option<String>,

    /// Bitcoin Address. Populated for internal Bitcoin address payments
    #[serde(skip_serializing_if = "Option::is_none")]
    #[schema(example = "bc1q...")]
    pub btc_address: Option<String>,

    /// Payment hash. Populated for internal bolt11 payments
    #[serde(skip_serializing_if = "Option::is_none")]
    #[schema(example = "b587c7f76339e3fb87ad2b...")]
    pub payment_hash: Option<String>,
}

#[derive(Clone, Debug, EnumString, Display, Deserialize, Serialize, PartialEq, Eq, Default, ToSchema)]
pub enum PaymentStatus {
    #[default]
    Pending,
    Settled,
    Failed,
}
