use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use serde_with::{serde_as, DisplayFromStr};
use strum_macros::{Display, EnumString};
use utoipa::{IntoParams, ToSchema};
use uuid::Uuid;

use crate::{Ledger, LnUrlSuccessAction, OrderDirection};

/// An outgoing payment, over Lightning, on-chain, or internal to the instance.
#[derive(Clone, Debug, Default, Deserialize, Serialize, ToSchema)]
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

/// Lightning-specific details of a payment.
#[derive(Clone, Debug, Default, Deserialize, Serialize, ToSchema)]
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

/// On-chain Bitcoin details of a payment.
#[derive(Clone, Debug, Default, Deserialize, Serialize, ToSchema)]
pub struct BtcPayment {
    /// Destination Bitcoin address. Populated for Bitcoin onchain payments.
    pub address: String,

    /// Transaction ID for on-chain payments.
    pub txid: String,

    /// Bitcoin block height where the transaction was confirmed.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub block_height: Option<u32>,
}

/// Details of a payment settled internally between wallets on the same instance.
#[derive(Clone, Debug, Default, Deserialize, Serialize, ToSchema)]
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

/// Lifecycle status of a payment.
#[derive(Clone, Debug, EnumString, Display, Deserialize, Serialize, PartialEq, Eq, Default, ToSchema)]
pub enum PaymentStatus {
    #[default]
    Pending,
    Settled,
    Failed,
}

/// Send Payment Request
#[derive(Debug, Deserialize, Clone, ToSchema, Serialize)]
pub struct SendPaymentRequest {
    /// Wallet ID to pay from. Required by admin endpoints; derived from the path on wallet-scoped endpoints.
    pub wallet_id: Option<Uuid>,

    /// Recipient. Can be a Bolt11 invoice, LNURL or LN Address.
    #[schema(example = "hello@numeraire.tech")]
    pub input: String,

    /// Amount in millisatoshis. Only necessary if the input does not specify an amount (empty Bolt11, LNURL or LN Address)
    pub amount_msat: Option<u64>,
    /// Comment of the payment. Visible by the recipient for LNURL payments
    pub comment: Option<String>,
}

/// Fee quote for a prospective outgoing payment.
///
/// `estimated_fee_msat` is the route or transaction fee expected at quote time.
/// It can be absent when the Lightning node cannot find a graph route while the
/// configured payment policy still permits an execution attempt. The maximum is
/// the hard cap passed to the Lightning provider (or the prepared on-chain fee).
#[derive(Clone, Debug, Deserialize, Serialize, PartialEq, Eq, ToSchema)]
pub struct PaymentFeeEstimate {
    /// Ledger selected for this payment input.
    pub ledger: Ledger,

    /// Amount delivered to the recipient, in millisatoshis.
    pub amount_msat: u64,

    /// Provider-derived fee expected at quote time, in millisatoshis.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub estimated_fee_msat: Option<u64>,

    /// Lightning execution cap or current prepared on-chain fee, in millisatoshis.
    pub maximum_fee_msat: u64,

    /// Expected amount plus fee, in millisatoshis.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub estimated_total_msat: Option<u64>,

    /// Amount plus the Lightning cap or current on-chain fee, in millisatoshis.
    pub maximum_total_msat: u64,
}

/// Payment query filter.
#[serde_as]
#[derive(Clone, Debug, Deserialize, Serialize, Default, IntoParams, ToSchema)]
pub struct PaymentFilter {
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
    pub status: Option<PaymentStatus>,
    /// Ledger
    pub ledger: Option<Ledger>,

    /// Lightning addresses
    #[schema(example = "donations@numeraire.tech")]
    pub ln_addresses: Option<Vec<String>>,

    /// Bitcoin addresses
    #[schema(example = "bc1q...")]
    pub btc_addresses: Option<Vec<String>>,

    /// Direction of the ordering of results
    #[serde(default)]
    pub order_direction: OrderDirection,
}
