use crate::domains::event::BtcOutputEvent;

#[derive(Clone, Debug)]
pub struct BtcTransaction {
    pub txid: String,
    pub block_height: Option<u32>,
    pub outputs: Vec<BtcTransactionOutput>,
    /// True if we spent any of the inputs (outgoing tx), false if this is a deposit to our wallet
    pub is_outgoing: bool,
}

impl BtcTransaction {
    pub fn output_event(&self, output: &BtcTransactionOutput) -> BtcOutputEvent {
        BtcOutputEvent {
            txid: self.txid.clone(),
            output_index: output.output_index,
            address: Some(output.address.clone()),
            amount_sat: output.amount_sat,
            block_height: self.block_height,
        }
    }
}

#[derive(Clone, Debug)]
pub struct BtcTransactionOutput {
    pub output_index: u32,
    pub address: String,
    pub amount_sat: u64,
    pub is_ours: bool,
}

#[derive(Clone, Debug)]
pub struct BtcPreparedTransaction {
    pub txid: String,
    pub fee_sat: u64,
    pub psbt: String,
    pub locked_utxos: Vec<BtcLockedUtxo>,
}

#[derive(Clone, Debug)]
pub struct BtcLockedUtxo {
    pub id: String,
    pub txid: String,
    pub output_index: u32,
}
