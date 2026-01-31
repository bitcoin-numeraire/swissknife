use chrono::{DateTime, Utc};

use crate::domains::bitcoin::BtcNetwork;

#[derive(Debug, Clone)]
pub struct BtcOutputEvent {
    pub txid: String,
    pub output_index: u32,
    pub address: Option<String>,
    pub amount_sat: u64,
    pub timestamp: DateTime<Utc>,
    pub fee_sat: Option<u64>,
    pub block_height: Option<u32>,
    pub network: BtcNetwork,
}
