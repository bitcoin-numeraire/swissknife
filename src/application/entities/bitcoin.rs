use async_trait::async_trait;

use crate::{
    application::errors::BitcoinError,
    domains::bitcoin::{BitcoinAddressType, BitcoinNetwork, BitcoinTransaction},
};

#[async_trait]
pub trait BitcoinWallet: Sync + Send {
    async fn new_address(&self, address_type: BitcoinAddressType) -> Result<String, BitcoinError>;
    async fn send(&self, address: String, amount_sat: u64, fee_rate: Option<u32>) -> Result<String, BitcoinError>;
    async fn get_transaction(&self, txid: &str) -> Result<BitcoinTransaction, BitcoinError>;
    fn network(&self) -> BitcoinNetwork;
}
