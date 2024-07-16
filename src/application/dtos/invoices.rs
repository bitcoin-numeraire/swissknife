use std::time::Duration;

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use serde_with::{serde_as, DurationSeconds};
use utoipa::ToSchema;
use uuid::Uuid;

use crate::{
    application::entities::Ledger,
    domains::invoice::{Invoice, InvoiceStatus, LnInvoice},
};

/// New Invoice Request
#[derive(Debug, Deserialize, ToSchema)]
pub struct NewInvoiceRequest {
    /// User ID. Will be populated with your own ID by default
    pub wallet_id: Option<Uuid>,
    /// Amount in millisatoshis
    pub amount_msat: u64,
    /// Description of the invoice. Visible by the payer
    pub description: Option<String>,
    /// Expiration time in seconds
    pub expiry: Option<u32>,
}

#[derive(Serialize, ToSchema)]
pub struct InvoiceResponse {
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
    #[serde(skip_serializing_if = "Option::is_none")]
    /// Fees paid. Populated when a new channel is opened to receive the funds.
    pub fee_msat: Option<u64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    /// Payment time
    pub payment_time: Option<DateTime<Utc>>,
    /// Date of creation in database
    pub created_at: DateTime<Utc>,
    #[serde(skip_serializing_if = "Option::is_none")]
    /// Date of update in database
    pub updated_at: Option<DateTime<Utc>>,

    #[serde(skip_serializing_if = "Option::is_none")]
    /// Lightning details of the invoice
    pub ln_invoice: Option<LnInvoiceResponse>,
}

#[serde_as]
#[derive(Serialize, ToSchema)]
pub struct LnInvoiceResponse {
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

impl From<Invoice> for InvoiceResponse {
    fn from(invoice: Invoice) -> Self {
        InvoiceResponse {
            id: invoice.id,
            wallet_id: invoice.wallet_id,
            ln_address_id: invoice.ln_address_id,
            description: invoice.description,
            amount_msat: invoice.amount_msat,
            amount_received_msat: invoice.amount_received_msat,
            timestamp: invoice.timestamp,
            status: invoice.status,
            ledger: invoice.ledger,
            fee_msat: invoice.fee_msat,
            payment_time: invoice.payment_time,
            created_at: invoice.created_at,
            updated_at: invoice.updated_at,
            ln_invoice: invoice.ln_invoice.map(Into::into),
        }
    }
}

impl From<LnInvoice> for LnInvoiceResponse {
    fn from(invoice: LnInvoice) -> Self {
        LnInvoiceResponse {
            payment_hash: invoice.payment_hash,
            bolt11: invoice.bolt11,
            description_hash: invoice.description_hash,
            payee_pubkey: invoice.payee_pubkey,
            min_final_cltv_expiry_delta: invoice.min_final_cltv_expiry_delta,
            payment_secret: invoice.payment_secret,
            expiry: invoice.expiry,
            expires_at: invoice.expires_at,
        }
    }
}
