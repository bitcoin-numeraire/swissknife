use chrono::{DateTime, Utc};
use uuid::Uuid;

use crate::application::entities::Currency;

#[derive(Clone, Debug)]
#[allow(dead_code)]
pub struct BitcoinOutput {
    pub address: Option<String>,
    pub amount_sat: u64,
    pub output_index: Option<u32>,
}

#[derive(Clone, Debug)]
#[allow(dead_code)]
pub struct BitcoinTransaction {
    pub id: Uuid,
    pub txid: String,
    pub amount_sat: i64,
    pub fee_sat: Option<u64>,
    pub block_height: Option<u32>,
    pub timestamp: Option<DateTime<Utc>>,
    pub currency: Currency,
    pub outputs: Vec<BitcoinOutput>,
}
