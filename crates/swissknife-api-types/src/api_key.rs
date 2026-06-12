use chrono::{DateTime, Utc};
use uuid::Uuid;

use crate::Permission;

#[derive(Clone, Debug, Default)]
pub struct ApiKey {
    pub id: Uuid,
    pub user_id: String,
    pub name: String,
    pub key: Option<String>,
    pub key_hash: Vec<u8>,
    pub permissions: Vec<Permission>,
    pub description: Option<String>,
    pub created_at: DateTime<Utc>,
    pub expires_at: Option<DateTime<Utc>>,
}
