use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use utoipa::ToSchema;
use uuid::Uuid;

use crate::domains::user::{ApiKey, Permission};

/// Create API Key Request
#[derive(Deserialize, ToSchema)]
pub struct CreateApiKeyRequest {
    /// User ID. Will be populated with your own ID by default
    pub user_id: Option<String>,
    /// List of permissions for this API key
    pub permissions: Vec<Permission>,
    /// API key description
    pub description: Option<String>,
    /// Expiration time in seconds
    pub expiry: Option<u32>,
}

/// API Key Response
#[derive(Serialize, ToSchema)]
pub struct ApiKeyResponse {
    /// Internal ID
    pub id: Uuid,
    /// User ID
    pub user_id: String,
    /// API key (only returned once on creation, save it securely as it cannot be retrieved)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub key: Option<String>,
    /// List of permissions for this API key
    pub permissions: Vec<Permission>,
    /// API key description
    pub description: Option<String>,
    /// Date of creation in database
    pub created_at: DateTime<Utc>,
    /// Date of expiration
    pub expires_at: Option<DateTime<Utc>>,
}

impl From<ApiKey> for ApiKeyResponse {
    fn from(key: ApiKey) -> Self {
        ApiKeyResponse {
            id: key.id,
            user_id: key.user_id,
            key: key.key,
            permissions: key.permissions,
            description: key.description,
            created_at: key.created_at,
            expires_at: key.expires_at,
        }
    }
}
