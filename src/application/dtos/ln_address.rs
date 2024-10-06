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
    #[schema(example = "npub1m8pwckdf3...")]
    pub nostr_pubkey: Option<PublicKey>,
}
