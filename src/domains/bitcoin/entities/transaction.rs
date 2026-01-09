use chrono::{DateTime, Utc};

#[derive(Clone, Debug)]
pub struct BitcoinTransaction {
    pub txid: String,
    pub timestamp: Option<DateTime<Utc>>,
    pub fee_sat: Option<i64>,
    pub block_height: Option<i64>,
    pub confirmations: Option<i64>,
    pub outputs: Vec<BitcoinTransactionOutput>,
}

#[derive(Clone, Debug)]
pub struct BitcoinTransactionOutput {
    pub output_index: u32,
    pub address: Option<String>,
    pub amount_sat: i64,
    pub is_ours: bool,
}
