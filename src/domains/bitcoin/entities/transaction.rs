use chrono::{DateTime, Utc};

use crate::domains::bitcoin::BitcoinNetwork;

#[derive(Clone, Debug)]
pub struct BitcoinTransaction {
    pub txid: String,
    pub timestamp: Option<DateTime<Utc>>,
    pub fee_sat: Option<u64>,
    pub block_height: Option<u32>,
    pub confirmations: Option<u32>,
    pub outputs: Vec<BitcoinTransactionOutput>,
}

impl BitcoinTransaction {
    pub fn output_event(&self, output: &BitcoinTransactionOutput, network: BitcoinNetwork) -> BitcoinOutputEvent {
        BitcoinOutputEvent {
            txid: self.txid.clone(),
            output_index: output.output_index,
            address: output.address.clone(),
            amount_sat: output.amount_sat,
            timestamp: self.timestamp,
            fee_sat: self.fee_sat,
            block_height: self.block_height,
            confirmations: self.confirmations,
            network,
        }
    }
}

#[derive(Clone, Debug)]
pub struct BitcoinTransactionOutput {
    pub output_index: u32,
    pub address: Option<String>,
    pub amount_sat: u64,
    pub is_ours: bool,
}

#[derive(Clone, Debug)]
pub struct BitcoinOutputEvent {
    pub txid: String,
    pub output_index: u32,
    pub address: Option<String>,
    pub amount_sat: u64,
    pub timestamp: Option<DateTime<Utc>>,
    pub fee_sat: Option<u64>,
    pub block_height: Option<u32>,
    pub confirmations: Option<u32>,
    pub network: BitcoinNetwork,
}
