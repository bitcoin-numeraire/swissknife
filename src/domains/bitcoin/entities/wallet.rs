use async_trait::async_trait;

use crate::{
    application::errors::BitcoinError,
    domains::bitcoin::{
        BtcAddressType, BtcNetwork, BtcOutput, BtcPreparedTransaction, BtcTransaction, OnchainSyncBatch,
        OnchainSyncCursor,
    },
};

#[async_trait]
pub trait BitcoinWallet: Sync + Send {
    async fn new_address(&self, address_type: BtcAddressType) -> Result<String, BitcoinError>;
    async fn prepare_transaction(
        &self,
        address: String,
        amount_sat: u64,
        feerate_sat_vb: Option<u32>,
    ) -> Result<BtcPreparedTransaction, BitcoinError>;

    /// Signs and broadcasts the prepared transaction. Returns an optional txid
    /// if the real txid is only known after broadcast (e.g. Breez chain swaps).
    async fn sign_send_transaction(&self, prepared: &BtcPreparedTransaction) -> Result<Option<String>, BitcoinError>;
    async fn release_prepared_transaction(&self, prepared: &BtcPreparedTransaction) -> Result<(), BitcoinError>;
    async fn get_transaction(&self, txid: &str) -> Result<Option<BtcTransaction>, BitcoinError>;
    async fn synchronize(&self, cursor: Option<OnchainSyncCursor>) -> Result<OnchainSyncBatch, BitcoinError>;
    async fn get_output(
        &self,
        txid: &str,
        output_index: Option<u32>,
        address: Option<&str>,
        include_spent: bool,
    ) -> Result<Option<BtcOutput>, BitcoinError>;
    fn network(&self) -> BtcNetwork;
}
