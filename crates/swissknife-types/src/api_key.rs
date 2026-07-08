use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use serde_with::{serde_as, DisplayFromStr};
use utoipa::{IntoParams, ToSchema};
use uuid::Uuid;

use crate::{OrderDirection, Permission};

/// API Key
#[derive(Clone, Debug, Default, Deserialize, Serialize, ToSchema)]
pub struct ApiKey {
    /// Internal ID
    pub id: Uuid,
    /// Owning account ID
    pub account_id: Uuid,
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

/// Create API Key Request
#[derive(Deserialize, ToSchema, Serialize)]
pub struct CreateApiKeyRequest {
    /// Owning account ID.
    ///
    /// User-scoped endpoints populate this with your own account.
    pub account_id: Option<Uuid>,
    /// API key name
    pub name: String,
    /// List of permissions for this API key
    pub permissions: Vec<Permission>,
    /// API key description
    pub description: Option<String>,
    /// Expiration time in seconds
    pub expiry: Option<u32>,
}

/// API key query filter.
#[serde_as]
#[derive(Clone, Debug, Deserialize, Serialize, Default, IntoParams)]
pub struct ApiKeyFilter {
    /// Total amount of results to return
    #[serde_as(as = "Option<DisplayFromStr>")]
    pub limit: Option<u64>,
    /// Offset where to start returning results
    #[serde_as(as = "Option<DisplayFromStr>")]
    pub offset: Option<u64>,
    /// List of IDs
    pub ids: Option<Vec<Uuid>>,
    /// Owning account ID.
    ///
    /// User-scoped endpoints populate this from the authenticated account.
    pub account_id: Option<Uuid>,
    /// Direction of the ordering of results
    #[serde(default)]
    pub order_direction: OrderDirection,
}
