use async_trait::async_trait;

use crate::{
    application::errors::BitcoinError,
    domains::bitcoin::{
        BitcoinWallet, BtcAddressType, BtcNetwork, BtcOutput, BtcPreparedTransaction, BtcTransaction, OnchainSyncBatch,
        OnchainSyncCursor,
    },
};

/// An explicit absence of a compatible chain adapter. In particular, never
/// represent an unsupported history query as an empty successful sync.
pub struct UnavailableBitcoinWallet {
    pub network: BtcNetwork,
    pub reason: &'static str,
}

#[async_trait]
impl BitcoinWallet for UnavailableBitcoinWallet {
    async fn new_address(&self, _address_type: BtcAddressType) -> Result<String, BitcoinError> {
        Err(BitcoinError::Unsupported(self.reason))
    }
    async fn prepare_transaction(
        &self,
        _address: String,
        _amount_sat: u64,
        _feerate_sat_vb: Option<u32>,
    ) -> Result<BtcPreparedTransaction, BitcoinError> {
        Err(BitcoinError::Unsupported(self.reason))
    }
    async fn sign_send_transaction(&self, _prepared: &BtcPreparedTransaction) -> Result<Option<String>, BitcoinError> {
        Err(BitcoinError::Unsupported(self.reason))
    }
    async fn release_prepared_transaction(&self, _prepared: &BtcPreparedTransaction) -> Result<(), BitcoinError> {
        Err(BitcoinError::Unsupported(self.reason))
    }
    async fn get_transaction(&self, _txid: &str) -> Result<Option<BtcTransaction>, BitcoinError> {
        Err(BitcoinError::Unsupported(self.reason))
    }
    async fn synchronize(&self, _cursor: Option<OnchainSyncCursor>) -> Result<OnchainSyncBatch, BitcoinError> {
        Err(BitcoinError::Unsupported(self.reason))
    }
    async fn get_output<'a>(
        &self,
        _txid: &str,
        _output_index: Option<u32>,
        _address: Option<&'a str>,
        _include_spent: bool,
    ) -> Result<Option<BtcOutput>, BitcoinError> {
        Err(BitcoinError::Unsupported(self.reason))
    }
    fn network(&self) -> BtcNetwork {
        self.network
    }
}
