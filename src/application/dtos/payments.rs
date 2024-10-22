use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use utoipa::ToSchema;
use uuid::Uuid;

use crate::{
    application::entities::{Currency, Ledger},
    domains::{
        lnurl::LnUrlSuccessAction,
        payment::{Payment, PaymentStatus},
    },
};

/// Send Payment Request
#[derive(Debug, Deserialize, Clone, ToSchema)]
pub struct SendPaymentRequest {
    /// Wallet ID. Will be populated with your own ID by default
    pub wallet_id: Option<Uuid>,

    /// Recipient. Can be a Bolt11 invoice, LNURL or LN Address. Keysend and On-chain payments not yet supported
    #[schema(example = "hello@numeraire.tech")]
    pub input: String,

    /// Amount in millisatoshis. Only necessary if the input does not specify an amount (empty Bolt11, LNURL or LN Address)
    pub amount_msat: Option<u64>,
    /// Comment of the payment. Visible by the recipient for LNURL payments
    pub comment: Option<String>,
}

/// Send On-chain Payment Request
#[derive(Debug, Deserialize, ToSchema)]
pub struct SendOnchainPaymentRequest {
    /// Amount in millisatoshis
    #[schema(example = 100000000)]
    pub amount_msat: u64,

    /// Recipient Bitcoin address
    #[schema(example = "bc1q7jys2n3jjf9t25r6ut369taap8v38pgqekq8v4")]
    pub recipient_address: String,

    /// Fee rate in sats/vb
    #[schema(example = "8")]
    pub feerate: u32,
}

#[derive(Serialize, ToSchema)]
pub struct PaymentResponse {
    /// Internal ID
    pub id: Uuid,

    /// Wallet ID
    pub wallet_id: Uuid,

    /// Lightning Address. Populated when sending to a LN Address
    #[serde(skip_serializing_if = "Option::is_none")]
    #[schema(example = "hello@numeraire.tech")]
    pub ln_address: Option<String>,

    /// Payment hash
    #[serde(skip_serializing_if = "Option::is_none")]
    #[schema(example = "b587c7f76339e3fb87ad2b...")]
    pub payment_hash: Option<String>,

    /// Payment Preimage
    #[serde(skip_serializing_if = "Option::is_none")]
    pub payment_preimage: Option<String>,

    /// Error message
    #[serde(skip_serializing_if = "Option::is_none")]
    #[schema(example = "failed to pay error message")]
    pub error: Option<String>,

    /// Amount in millisatoshis.
    pub amount_msat: u64,

    #[serde(skip_serializing_if = "Option::is_none")]
    /// Fees paid. Populated when a new channel is opened to receive the funds
    pub fee_msat: Option<u64>,

    /// Currency
    pub currency: Currency,

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

    /// Metadata
    #[serde(skip_serializing_if = "Option::is_none")]
    pub metadata: Option<String>,

    /// Success Action. Populated when sending to a LNURL or LN Address
    #[serde(skip_serializing_if = "Option::is_none")]
    pub success_action: Option<LnUrlSuccessAction>,

    /// Date of creation in database
    pub created_at: DateTime<Utc>,

    /// Date of update in database
    #[serde(skip_serializing_if = "Option::is_none")]
    pub updated_at: Option<DateTime<Utc>>,
}

impl From<Payment> for PaymentResponse {
    fn from(payment: Payment) -> Self {
        PaymentResponse {
            id: payment.id,
            wallet_id: payment.wallet_id,
            ln_address: payment.ln_address,
            payment_hash: payment.payment_hash,
            payment_preimage: payment.payment_preimage,
            error: payment.error,
            amount_msat: payment.amount_msat,
            fee_msat: payment.fee_msat,
            ledger: payment.ledger,
            currency: payment.currency,
            payment_time: payment.payment_time,
            status: payment.status,
            description: payment.description,
            metadata: payment.metadata,
            success_action: payment.success_action,
            created_at: payment.created_at,
            updated_at: payment.updated_at,
        }
    }
}
