use chrono::{DateTime, FixedOffset};
use serde::Serialize;
use uuid::Uuid;

#[derive(Serialize, Debug)]
pub struct LightningAddress {
    pub id: Uuid,
    pub user_id: String,
    pub username: String,
    pub active: bool,
    pub created_at: DateTime<FixedOffset>,
    pub updated_at: Option<DateTime<FixedOffset>>,
    pub deleted_at: Option<DateTime<FixedOffset>>,
}
