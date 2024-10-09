use nostr_sdk::PublicKey;
use serde::Deserialize;
use utoipa::ToSchema;
use uuid::Uuid;

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
