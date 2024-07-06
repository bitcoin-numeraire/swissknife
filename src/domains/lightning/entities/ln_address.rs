use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use serde_with::{serde_as, DisplayFromStr};
use utoipa::{IntoParams, ToSchema};
use uuid::Uuid;

/// Lightning Address
#[derive(Clone, Debug, Serialize, ToSchema)]
pub struct LnAddress {
    /// Internal ID
    pub id: Uuid,
    /// User ID
    pub user_id: String,
    /// Username
    pub username: String,
    /// Active status. Inactive addresses cannot receive funds
    pub active: bool,
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
    /// User ID. Automatically populated with your user ID
    pub user_id: Option<String>,
    /// Status
    pub username: Option<String>,
}
