use async_trait::async_trait;

use crate::{
    application::errors::BitcoinError,
    domains::bitcoin::{BtcAddressType, BtcNetwork, BtcOutput, BtcTransaction},
};

#[derive(Clone, Debug, Default)]
pub struct BtcLockedUtxo {
    pub id: String,
    pub txid: String,
    pub output_index: u32,
}

#[derive(Clone, Debug, Default)]
pub struct BtcPreparedTransaction {
    pub txid: String,
    pub fee_sat: u64,
    pub raw_tx: Option<Vec<u8>>,
    pub locked_utxos: Vec<BtcLockedUtxo>,
}

#[async_trait]
pub trait BitcoinWallet: Sync + Send {
    async fn new_address(&self, address_type: BtcAddressType) -> Result<String, BitcoinError>;
    async fn send(&self, address: String, amount_sat: u64, fee_rate: Option<u32>) -> Result<String, BitcoinError>;
    async fn prepare_send(
        &self,
        address: String,
        amount_sat: u64,
        fee_rate: Option<u32>,
        lock_id: Option<String>,
    ) -> Result<BtcPreparedTransaction, BitcoinError>;
    async fn broadcast_transaction(&self, prepared: &BtcPreparedTransaction) -> Result<String, BitcoinError>;
    async fn release_prepared_transaction(&self, prepared: &BtcPreparedTransaction) -> Result<(), BitcoinError>;
    async fn get_transaction(&self, txid: &str) -> Result<BtcTransaction, BitcoinError>;
    async fn get_output(
        &self,
        txid: &str,
        output_index: Option<u32>,
        address: Option<&str>,
        include_spent: bool,
    ) -> Result<Option<BtcOutput>, BitcoinError>;
    fn network(&self) -> BtcNetwork;
}
