use chrono::{DateTime, Utc};
use uuid::Uuid;

use crate::application::entities::Currency;

#[derive(Clone, Debug)]
#[allow(dead_code)]
pub struct BitcoinOutput {
    pub id: Uuid,
    pub outpoint: String,
    pub txid: String,
    pub output_index: u32,
    pub address: Option<String>,
    pub amount_sat: i64,
    pub fee_sat: Option<i64>,
    pub block_height: Option<u32>,
    pub timestamp: Option<DateTime<Utc>>,
    pub currency: Currency,
    pub created_at: DateTime<Utc>,
    pub updated_at: Option<DateTime<Utc>>,
}
