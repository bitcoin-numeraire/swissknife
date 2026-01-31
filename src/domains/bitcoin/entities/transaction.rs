use chrono::{DateTime, Utc};

use crate::domains::{bitcoin::BtcNetwork, event::BtcOutputEvent};

#[derive(Clone, Debug)]
pub struct BtcTransaction {
    pub txid: String,
    pub timestamp: Option<DateTime<Utc>>,
    pub fee_sat: Option<u64>,
    pub block_height: Option<u32>,
    pub outputs: Vec<BtcTransactionOutput>,
}

impl BtcTransaction {
    pub fn output_event(&self, output: &BtcTransactionOutput, network: BtcNetwork) -> BtcOutputEvent {
        BtcOutputEvent {
            txid: self.txid.clone(),
            output_index: output.output_index,
            address: output.address.clone(),
            amount_sat: output.amount_sat,
            timestamp: self.timestamp.unwrap_or(Utc::now()),
            fee_sat: self.fee_sat,
            block_height: self.block_height,
            network,
        }
    }
}

#[derive(Clone, Debug)]
pub struct BtcTransactionOutput {
    pub output_index: u32,
    pub address: Option<String>,
    pub amount_sat: u64,
    pub is_ours: bool,
}
