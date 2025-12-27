use chrono::{DateTime, Utc};
use uuid::Uuid;

#[derive(Clone, Debug)]
#[allow(dead_code)]
pub struct BitcoinAddress {
    pub id: Uuid,
    pub wallet_id: Uuid,
    pub address: String,
    pub used: bool,
    pub derivation_index: Option<i64>,
    pub created_at: DateTime<Utc>,
    pub updated_at: Option<DateTime<Utc>>,
}
