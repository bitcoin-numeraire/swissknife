use crate::domains::bitcoin::BtcOutput;

#[derive(Debug, Clone, Default)]
pub struct OnchainDepositEvent {
    pub txid: String,
    pub output_index: u32,
    pub address: String,
    pub amount_sat: u64,
    pub block_height: Option<u32>,
}

impl From<BtcOutput> for OnchainDepositEvent {
    fn from(output: BtcOutput) -> Self {
        OnchainDepositEvent {
            txid: output.txid,
            output_index: output.output_index,
            address: output.address,
            amount_sat: output.amount_sat,
            block_height: output.block_height,
        }
    }
}

#[derive(Debug, Clone, Default)]
pub struct OnchainWithdrawalEvent {
    pub txid: String,
    pub block_height: Option<u32>,
}
