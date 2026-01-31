use crate::domains::bitcoin::{BtcNetwork, BtcOutput};

#[derive(Debug, Clone)]
pub struct BtcOutputEvent {
    pub txid: String,
    pub output_index: u32,
    pub address: Option<String>,
    pub amount_sat: u64,
    pub block_height: Option<u32>,
    pub network: BtcNetwork,
}

impl From<BtcOutput> for BtcOutputEvent {
    fn from(output: BtcOutput) -> Self {
        BtcOutputEvent {
            txid: output.txid,
            output_index: output.output_index,
            address: Some(output.address),
            amount_sat: output.amount_sat,
            block_height: output.block_height,
            network: output.network,
        }
    }
}
