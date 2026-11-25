use async_trait::async_trait;

use crate::{
    application::errors::BitcoinError,
    domains::bitcoin::{
        BtcAddressType, BtcBroadcastTransaction, BtcNetwork, BtcOutput, BtcPreparedTransaction, BtcTransaction,
        OnchainSyncBatch, OnchainSyncCursor,
    },
};

#[cfg_attr(test, mockall::automock)]
#[async_trait]
pub trait BitcoinWallet: Sync + Send {
    async fn new_address(&self, address_type: BtcAddressType) -> Result<String, BitcoinError>;
    async fn prepare_transaction(
        &self,
        address: String,
        amount_sat: u64,
        feerate_sat_vb: Option<u32>,
    ) -> Result<BtcPreparedTransaction, BitcoinError>;

    /// Obtain the authoritative txid before publishing. This operation never broadcasts.
    async fn finalize_transaction(
        &self,
        prepared: &BtcPreparedTransaction,
    ) -> Result<BtcBroadcastTransaction, BitcoinError> {
        Ok(BtcBroadcastTransaction::Prepared(prepared.txid.clone()))
    }

    /// A lost reply cannot establish whether the node published the transaction.
    async fn broadcast_transaction(&self, transaction: &BtcBroadcastTransaction) -> Result<(), BitcoinError>;
    async fn release_prepared_transaction(&self, prepared: &BtcPreparedTransaction) -> Result<(), BitcoinError>;
    async fn get_transaction(&self, txid: &str) -> Result<Option<BtcTransaction>, BitcoinError>;
    async fn synchronize(&self, cursor: Option<OnchainSyncCursor>) -> Result<OnchainSyncBatch, BitcoinError>;
    async fn get_output<'a>(
        &self,
        txid: &str,
        output_index: Option<u32>,
        address: Option<&'a str>,
        include_spent: bool,
    ) -> Result<Option<BtcOutput>, BitcoinError>;
    fn network(&self) -> BtcNetwork;
}
