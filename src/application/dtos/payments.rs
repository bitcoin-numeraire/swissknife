use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use utoipa::ToSchema;
use uuid::Uuid;

use crate::{
    application::{
        dtos::BtcOutputResponse,
        entities::{Currency, Ledger},
    },
    domains::{
        lnurl::LnUrlSuccessAction,
        payment::{BtcPayment, InternalPayment, LnPayment, Payment, PaymentStatus},
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

#[derive(Serialize, ToSchema)]
pub struct PaymentResponse {
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

    /// Date of creation in database
    pub created_at: DateTime<Utc>,

    /// Date of update in database
    #[serde(skip_serializing_if = "Option::is_none")]
    pub updated_at: Option<DateTime<Utc>>,

    /// Lightning payment details
    #[serde(skip_serializing_if = "Option::is_none")]
    pub lightning: Option<LnPaymentResponse>,

    /// Bitcoin on-chain payment details
    #[serde(skip_serializing_if = "Option::is_none")]
    pub bitcoin: Option<BtcPaymentResponse>,

    /// Internal payment details
    #[serde(skip_serializing_if = "Option::is_none")]
    pub internal: Option<InternalPaymentResponse>,
}

#[derive(Serialize, ToSchema)]
pub struct LnPaymentResponse {
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

#[derive(Serialize, ToSchema)]
pub struct BtcPaymentResponse {
    /// Destination Bitcoin address. Populated for Bitcoin onchain payments.
    pub address: String,

    /// Transaction ID for on-chain payments.
    pub txid: String,

    /// Bitcoin Output
    #[serde(skip_serializing_if = "Option::is_none")]
    pub output: Option<BtcOutputResponse>,
}

#[derive(Serialize, ToSchema)]
pub struct InternalPaymentResponse {
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

impl From<Payment> for PaymentResponse {
    fn from(payment: Payment) -> Self {
        PaymentResponse {
            id: payment.id,
            wallet_id: payment.wallet_id,
            error: payment.error,
            amount_msat: payment.amount_msat,
            fee_msat: payment.fee_msat,
            ledger: payment.ledger,
            currency: payment.currency,
            payment_time: payment.payment_time,
            status: payment.status,
            description: payment.description,
            created_at: payment.created_at,
            updated_at: payment.updated_at,
            lightning: payment.lightning.map(Into::into),
            bitcoin: payment.bitcoin.map(Into::into),
            internal: payment.internal.map(Into::into),
        }
    }
}

impl From<LnPayment> for LnPaymentResponse {
    fn from(payment: LnPayment) -> Self {
        LnPaymentResponse {
            ln_address: payment.ln_address,
            payment_hash: payment.payment_hash,
            payment_preimage: payment.payment_preimage,
            metadata: payment.metadata,
            success_action: payment.success_action,
        }
    }
}

impl From<BtcPayment> for BtcPaymentResponse {
    fn from(payment: BtcPayment) -> Self {
        BtcPaymentResponse {
            address: payment.address,
            txid: payment.txid,
            output: payment.output.map(Into::into),
        }
    }
}

impl From<InternalPayment> for InternalPaymentResponse {
    fn from(payment: InternalPayment) -> Self {
        InternalPaymentResponse {
            ln_address: payment.ln_address,
            btc_address: payment.btc_address,
            payment_hash: payment.payment_hash,
        }
    }
}
