//! Request types — the edge inputs decoded from the wire. Unlike entities,
//! these never reach the use cases as-is; they are distinct contract types.

use nostr::PublicKey;
use serde::Deserialize;
use utoipa::{IntoParams, ToSchema};
use uuid::Uuid;

use crate::{BtcAddressType, Permission};

/// Sign Up Request
#[derive(Debug, Deserialize, ToSchema)]
pub struct SignUpRequest {
    /// User password
    #[schema(example = "password")]
    pub password: String,
}

/// Sign In Request
#[derive(Debug, Deserialize, ToSchema)]
pub struct SignInRequest {
    /// User password
    #[schema(example = "password")]
    pub password: String,
}

/// Register Wallet Request
#[derive(Debug, Deserialize, Clone, ToSchema)]
pub struct RegisterWalletRequest {
    /// User ID. Should ideally be registered in your Auth provider.
    pub user_id: String,
}

/// New Invoice Request
#[derive(Deserialize, ToSchema)]
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

/// Create API Key Request
#[derive(Deserialize, ToSchema)]
pub struct CreateApiKeyRequest {
    /// User ID. Will be populated with your own ID by default
    pub user_id: Option<String>,
    /// API key name
    pub name: String,
    /// List of permissions for this API key
    pub permissions: Vec<Permission>,
    /// API key description
    pub description: Option<String>,
    /// Expiration time in seconds
    pub expiry: Option<u32>,
}

/// New Bitcoin Address Request
#[derive(Deserialize, ToSchema)]
pub struct NewBtcAddressRequest {
    /// User ID. Will be populated with your own ID by default
    pub wallet_id: Option<Uuid>,

    /// Address type
    #[serde(rename = "type")]
    pub address_type: Option<BtcAddressType>,
}

/// Register Lightning Address Request
#[derive(Debug, Deserialize, ToSchema)]
pub struct RegisterLnAddressRequest {
    /// Wallet ID. Will be populated with your own ID by default
    pub wallet_id: Option<Uuid>,

    /// Username such as `username@domain`
    pub username: String,

    /// Nostr enabled
    #[serde(default)]
    pub allows_nostr: bool,

    /// Nostr public key
    #[schema(value_type = Option<String>, example = "npub1m8pwckdf3...")]
    pub nostr_pubkey: Option<PublicKey>,
}

/// Update Lightning Address Request
#[derive(Debug, Deserialize, ToSchema)]
pub struct UpdateLnAddressRequest {
    /// Username such as `username@domain`
    pub username: Option<String>,

    /// Active status
    #[serde(default)]
    pub active: Option<bool>,

    /// Nostr enabled
    #[serde(default)]
    pub allows_nostr: Option<bool>,

    /// Nostr public key
    #[schema(value_type = Option<String>, example = "npub1m8pwckdf3...")]
    pub nostr_pubkey: Option<PublicKey>,
}

/// LNURL-pay callback query parameters
#[derive(Debug, Deserialize, IntoParams)]
pub struct LNUrlpInvoiceQueryParams {
    /// Amount in millisatoshis
    pub amount: u64,
    /// Optional comment for the recipient
    pub comment: Option<String>,
}

/// Nostr NIP-05 query parameters
#[derive(Debug, Deserialize, IntoParams)]
pub struct NostrNIP05QueryParams {
    /// Username to query
    #[serde(default)]
    pub name: String,
}
