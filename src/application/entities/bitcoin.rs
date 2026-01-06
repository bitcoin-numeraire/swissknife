use async_trait::async_trait;

use crate::{
    application::errors::BitcoinError,
    domains::bitcoin::{BitcoinAddressType, BitcoinBalance, BitcoinNetwork, BitcoinOutput},
};

#[async_trait]
pub trait BitcoinWallet: Sync + Send {
    async fn new_address(&self, address_type: BitcoinAddressType) -> Result<String, BitcoinError>;
    async fn balance(&self) -> Result<BitcoinBalance, BitcoinError>;
    async fn send(&self, address: String, amount_sat: u64, fee_rate: Option<u32>) -> Result<String, BitcoinError>;
    async fn list_outputs(&self) -> Result<Vec<BitcoinOutput>, BitcoinError>;
    async fn network(&self) -> Result<BitcoinNetwork, BitcoinError>;
}
