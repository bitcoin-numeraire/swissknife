use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use super::pagination::PaginationFilter;

#[derive(Clone, Debug, Serialize)]
pub struct LightningAddress {
    pub id: Uuid,
    pub user_id: String,
    pub username: String,
    pub active: bool,
    pub created_at: DateTime<Utc>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub updated_at: Option<DateTime<Utc>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub deleted_at: Option<DateTime<Utc>>,
}

#[derive(Clone, Debug, Deserialize, Serialize, Default)]
pub struct LightningAddressFilter {
    #[serde(flatten)]
    pub pagination: PaginationFilter,
    pub id: Option<Uuid>,
    pub user_id: Option<String>,
    pub username: Option<String>,
}
