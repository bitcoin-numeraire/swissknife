use chrono::{DateTime, Utc};
use nostr::PublicKey;
use serde::Serialize;
use utoipa::ToSchema;
use uuid::Uuid;

/// Lightning Address
#[derive(Clone, Debug, Serialize, ToSchema)]
pub struct LnAddress {
    /// Internal ID
    pub id: Uuid,
    /// Wallet ID
    pub wallet_id: Uuid,
    /// Username
    pub username: String,
    /// Active status. Inactive addresses cannot receive funds
    pub active: bool,
    /// Nostr enabled
    pub allows_nostr: bool,
    /// Nostr Public key
    #[serde(skip_serializing_if = "Option::is_none")]
    #[schema(value_type = Option<String>, example = "d9c2ec59a98c...")]
    pub nostr_pubkey: Option<PublicKey>,
    /// Date of creation in database
    pub created_at: DateTime<Utc>,
    #[serde(skip_serializing_if = "Option::is_none")]
    /// Date of update in database
    pub updated_at: Option<DateTime<Utc>>,
}
