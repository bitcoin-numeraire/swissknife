use chrono::{DateTime, Utc};
use nostr_sdk::PublicKey;
use serde::{Deserialize, Serialize};
use serde_with::{serde_as, DisplayFromStr};
use utoipa::{IntoParams, ToSchema};
use uuid::Uuid;

use crate::application::entities::OrderDirection;

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
    #[schema(example = "npub1m8pwckdf3...")]
    pub nostr_pubkey: Option<PublicKey>,
    /// Date of creation in database
    pub created_at: DateTime<Utc>,
    #[serde(skip_serializing_if = "Option::is_none")]
    /// Date of update in database
    pub updated_at: Option<DateTime<Utc>>,
}

#[serde_as]
#[derive(Clone, Debug, Deserialize, Serialize, Default, IntoParams)]
pub struct LnAddressFilter {
    /// Total amount of results to return
    #[serde_as(as = "Option<DisplayFromStr>")]
    pub limit: Option<u64>,
    /// Offset where to start returning results
    #[serde_as(as = "Option<DisplayFromStr>")]
    pub offset: Option<u64>,
    /// List of IDs
    pub ids: Option<Vec<Uuid>>,
    /// wallet ID. Automatically populated with your ID
    pub wallet_id: Option<Uuid>,
    /// Username
    pub username: Option<String>,
    /// Active
    pub active: Option<bool>,
    /// Direction of the ordering of results
    #[serde(default)]
    pub order_direction: OrderDirection,
}
