use chrono::{DateTime, Utc};
use serde::Serialize;
use uuid::Uuid;

#[derive(Debug, Serialize)]
pub struct LNURLp {
    pub callback: String, // The URL from LN SERVICE which will accept the pay request parameters
    pub max_sendable: u64, // Max amount in milli-satoshis LN SERVICE is willing to receive
    pub min_sendable: u64, // Min amount in milli-satoshis LN SERVICE is willing to receive, can not be less than 1 or more than `maxSendable`
    pub metadata: String, // Metadata json which must be presented as raw string here, this is required to pass signature verification at a later step
    pub comment_allowed: Option<u8>, // Optional number of characters accepted for the `comment` query parameter on subsequent callback, defaults to 0 if not provided. (no comment allowed)
    pub withdraw_link: Option<String>, // Optional lnurl-withdraw link
    pub tag: String,                 // Type of LNURL
}

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
