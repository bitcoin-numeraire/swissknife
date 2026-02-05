use std::sync::Arc;

use async_trait::async_trait;
use tracing::{debug, info, trace};
use uuid::Uuid;

use crate::{
    application::{
        entities::{AppStore, Currency},
        errors::{ApplicationError, DataError},
    },
    domains::{
        bitcoin::{BitcoinWallet, BtcAddressFilter, OnchainSyncCursor, OnchainTransaction},
        event::EventUseCases,
        system::SystemUseCases,
    },
};

use super::{BitcoinUseCases, BtcAddress, BtcAddressType};

pub struct BitcoinService {
    store: AppStore,
    wallet: Arc<dyn BitcoinWallet>,
    address_type: BtcAddressType,
    events: Arc<dyn EventUseCases>,
    system: Arc<dyn SystemUseCases>,
}

impl BitcoinService {
    pub fn new(
        store: AppStore,
        wallet: Arc<dyn BitcoinWallet>,
        address_type: BtcAddressType,
        events: Arc<dyn EventUseCases>,
        system: Arc<dyn SystemUseCases>,
    ) -> Self {
        Self {
            store,
            wallet,
            address_type,
            events,
            system,
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

    async fn sync(&self) -> Result<u32, ApplicationError> {
        trace!("Synchronizing on-chain bitcoin transactions");

        let mut cursor = self.system.get_onchain_cursor().await?;
        if cursor.is_none() {
            let output_height = self.store.btc_output.max_block_height().await?;
            let payment_height = self.store.payment.max_btc_block_height().await?;
            let start_height = output_height.or(payment_height).unwrap_or(0);
            cursor = Some(OnchainSyncCursor::BlockHeight(start_height));
        }

        let result = self.wallet.synchronize(cursor).await?;
        let currency: Currency = self.wallet.network().into();

        let mut synced = 0;

        for transaction in result.events {
            match transaction {
                OnchainTransaction::Deposit(output) => {
                    self.events.onchain_deposit(output.into(), currency.clone()).await?;
                    synced += 1;
                }
                OnchainTransaction::Withdrawal(event) => {
                    self.events.onchain_withdrawal(event).await?;
                    synced += 1;
                }
            }
        }

        if let Some(next_cursor) = result.next_cursor {
            self.system.set_onchain_cursor(next_cursor).await?;
        }

        debug!(synced, "On-chain bitcoin transactions synchronized successfully");
        Ok(synced)
    }
}
