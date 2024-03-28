use chrono::{DateTime, Utc};
use serde::Serialize;
use uuid::Uuid;

#[derive(Serialize, Debug)]
pub struct LightningAddress {
    pub id: Uuid,
    pub user_id: String,
    pub username: String,
    pub active: bool,
    pub created_at: DateTime<Utc>,
    pub updated_at: Option<DateTime<Utc>>,
    pub deleted_at: Option<DateTime<Utc>>,
}

#[derive(Serialize, Debug)]
pub struct UserBalance {
    pub received_msat: i64,
    pub sent_msat: i64,
    pub available_msat: i64,
}
