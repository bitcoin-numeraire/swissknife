use std::sync::Arc;

use async_trait::async_trait;
use tracing::{debug, info, trace};
use uuid::Uuid;

use crate::{
    application::{
        entities::AppStore,
        errors::{ApplicationError, DataError},
    },
    domains::bitcoin::{BitcoinWallet, BtcAddressFilter},
};

use super::{BitcoinUseCases, BtcAddress, BtcAddressType};

pub struct BitcoinService {
    store: AppStore,
    wallet: Arc<dyn BitcoinWallet>,
    address_type: BtcAddressType,
}

impl BitcoinService {
    pub fn new(store: AppStore, wallet: Arc<dyn BitcoinWallet>, address_type: BtcAddressType) -> Self {
        Self {
            store,
            wallet,
            address_type,
        }
    }
}

#[async_trait]
impl BitcoinUseCases for BitcoinService {
    async fn new_deposit_address(
        &self,
        wallet_id: Uuid,
        address_type: Option<BtcAddressType>,
    ) -> Result<BtcAddress, ApplicationError> {
        let address_type = address_type.unwrap_or(self.address_type);

        trace!(%wallet_id, %address_type, "Fetching current bitcoin deposit address");

        if let Some(address) = self
            .store
            .btc_address
            .find_by_wallet_unused(wallet_id, address_type)
            .await?
        {
            return Ok(address);
        }

        let address = self.wallet.new_address(address_type).await?;

        let btc_address = self.store.btc_address.insert(wallet_id, &address, address_type).await?;

        info!(%wallet_id, address = %btc_address.address, "New bitcoin deposit address issued");

        Ok(btc_address)
    }

    async fn get_address(&self, id: Uuid) -> Result<BtcAddress, ApplicationError> {
        trace!(%id, "Fetching Bitcoin address");

        let address = self
            .store
            .btc_address
            .find(id)
            .await?
            .ok_or_else(|| DataError::NotFound("Bitcoin address not found.".to_string()))?;

        debug!(%id, "Bitcoin address fetched successfully");
        Ok(address)
    }

    async fn list_addresses(&self, filter: BtcAddressFilter) -> Result<Vec<BtcAddress>, ApplicationError> {
        trace!(?filter, "Listing Bitcoin addresses");

        let addresses = self.store.btc_address.find_many(filter.clone()).await?;

        debug!(?filter, "Bitcoin addresses listed successfully");
        Ok(addresses)
    }

    async fn delete_address(&self, id: Uuid) -> Result<(), ApplicationError> {
        debug!(%id, "Deleting Bitcoin address");

        let n_deleted = self
            .store
            .btc_address
            .delete_many(BtcAddressFilter {
                ids: Some(vec![id]),
                ..Default::default()
            })
            .await?;

        if n_deleted == 0 {
            return Err(DataError::NotFound("Bitcoin address not found.".to_string()).into());
        }

        info!(%id, "Bitcoin address deleted successfully");
        Ok(())
    }

    async fn delete_many_addresses(&self, filter: BtcAddressFilter) -> Result<u64, ApplicationError> {
        debug!(?filter, "Deleting bitcoin addresses");

        let n_deleted = self.store.btc_address.delete_many(filter.clone()).await?;

        info!(?filter, n_deleted, "Bitcoin addresses deleted successfully");
        Ok(n_deleted)
    }
}
