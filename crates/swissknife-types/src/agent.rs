use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use utoipa::ToSchema;
use uuid::Uuid;

/// Independent permissions for a single wallet.
#[derive(Clone, Copy, Debug, Default, Deserialize, Serialize, ToSchema)]
#[serde(deny_unknown_fields)]
pub struct WalletCapabilities {
    #[serde(default)]
    pub read: bool,
    #[serde(default)]
    pub receive: bool,
    #[serde(default)]
    pub spend: bool,
    /// Save, rename and remove address-book entries for this wallet.
    #[serde(default)]
    pub manage_contacts: bool,
}

#[derive(Clone, Deserialize, Serialize, ToSchema)]
#[serde(deny_unknown_fields)]
pub struct CreateAgentKey {
    pub name: String,
    pub capabilities: WalletCapabilities,
    /// UTC expiry, up to one year from issuance.
    pub expires_at: DateTime<Utc>,
}

#[derive(Clone, Deserialize, Serialize, ToSchema)]
pub struct AgentKey {
    pub id: Uuid,
    pub wallet_id: Uuid,
    pub name: String,
    pub capabilities: WalletCapabilities,
    pub expires_at: DateTime<Utc>,
    pub created_at: DateTime<Utc>,
}

#[derive(Deserialize, Serialize, ToSchema)]
pub struct IssuedAgentKey {
    #[serde(flatten)]
    pub metadata: AgentKey,
    /// Secret shown once at creation. Send it in the Wallet-Key header.
    pub key: String,
}
