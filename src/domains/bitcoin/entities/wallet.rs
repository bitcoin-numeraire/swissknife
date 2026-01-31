use async_trait::async_trait;

use crate::{
    application::errors::BitcoinError,
    domains::bitcoin::{BtcAddressType, BtcNetwork, BtcOutput, BtcTransaction},
};

#[async_trait]
pub trait BitcoinWallet: Sync + Send {
    async fn new_address(&self, address_type: BtcAddressType) -> Result<String, BitcoinError>;
    async fn send(&self, address: String, amount_sat: u64, fee_rate: Option<u32>) -> Result<String, BitcoinError>;
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
