use chrono::{DateTime, Utc};
use serde::Serialize;
use utoipa::ToSchema;
use uuid::Uuid;

use crate::Permission;

/// API Key
#[derive(Clone, Debug, Default, Serialize, ToSchema)]
pub struct ApiKey {
    /// Internal ID
    pub id: Uuid,
    /// User ID
    pub user_id: String,
    /// API key name
    pub name: String,
    /// API key (only returned once on creation, save it securely as it cannot be retrieved)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub key: Option<String>,
    /// Hashed API key. Internal only.
    #[serde(skip)]
    pub key_hash: Vec<u8>,
    /// List of permissions for this API key
    pub permissions: Vec<Permission>,
    /// API key description
    pub description: Option<String>,
    /// Date of creation in database
    pub created_at: DateTime<Utc>,
    /// Date of expiration
    pub expires_at: Option<DateTime<Utc>>,
}
