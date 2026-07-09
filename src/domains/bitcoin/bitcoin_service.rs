use std::sync::Arc;

use async_trait::async_trait;
use tracing::{debug, info, trace};
use uuid::Uuid;

use crate::{
    application::{
        composition::AppStore,
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
        trace!("Synchronizing on-chain bitcoin transactions...");

        let mut cursor = self.system.get_onchain_cursor().await?;
        if cursor.is_none() {
            let output_height = self.store.btc_output.max_block_height().await?;
            let payment_height = self.store.payment.max_btc_block_height().await?;
            let start_height = output_height.or(payment_height).unwrap_or(0);
            cursor = Some(OnchainSyncCursor::BlockHeight(start_height));
        }

        let result = self.wallet.synchronize(cursor).await?;
        let mut synced = 0;

        for transaction in result.events {
            match transaction {
                OnchainTransaction::Deposit(output) => {
                    if self.events.onchain_deposit(output.into()).await? {
                        synced += 1;
                    }
                }
                OnchainTransaction::Withdrawal(event) => {
                    if self.events.onchain_withdrawal(event).await? {
                        synced += 1;
                    }
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

#[cfg(test)]
mod tests {
    use chrono::Utc;

    use crate::{
        application::composition::MockAppStoreBuilder,
        domains::{
            bitcoin::{BtcNetwork, BtcOutput, MockBitcoinWallet, OnchainSyncBatch, OnchainTransaction},
            event::{MockEventUseCases, OnchainWithdrawalEvent},
            system::MockSystemUseCases,
        },
    };

    use super::*;

    fn service(
        store: MockAppStoreBuilder,
        wallet: MockBitcoinWallet,
        events: MockEventUseCases,
        system: MockSystemUseCases,
    ) -> BitcoinService {
        BitcoinService::new(
            store.build(),
            Arc::new(wallet),
            BtcAddressType::P2wpkh,
            Arc::new(events),
            Arc::new(system),
        )
    }

    fn btc_address(wallet_id: Uuid, address: &str) -> BtcAddress {
        BtcAddress {
            id: Uuid::new_v4(),
            wallet_id,
            address: address.to_string(),
            used: false,
            address_type: BtcAddressType::P2wpkh,
            created_at: Utc::now(),
            updated_at: None,
        }
    }

    mod new_deposit_address {
        use super::*;

        mod when_an_unused_address_exists {
            use super::*;

            #[tokio::test]
            async fn reuses_it_without_deriving_a_new_one() {
                let wallet_id = Uuid::new_v4();

                let mut store = MockAppStoreBuilder::new();
                store
                    .btc_address
                    .expect_find_by_wallet_unused()
                    .withf(move |id, address_type| *id == wallet_id && *address_type == BtcAddressType::P2wpkh)
                    .times(1)
                    .returning(move |wallet_id, _| Ok(Some(btc_address(wallet_id, "bc1qreused"))));

                // wallet.new_address and btc_address.insert are intentionally not expected.
                let service = service(
                    store,
                    MockBitcoinWallet::new(),
                    MockEventUseCases::new(),
                    MockSystemUseCases::new(),
                );

                let address = service.new_deposit_address(wallet_id, None).await.unwrap();

                assert_eq!(address.address, "bc1qreused");
            }
        }

        mod when_no_unused_address_exists {
            use super::*;

            #[tokio::test]
            async fn derives_and_persists_a_new_address() {
                let wallet_id = Uuid::new_v4();

                let mut store = MockAppStoreBuilder::new();
                store
                    .btc_address
                    .expect_find_by_wallet_unused()
                    .times(1)
                    .returning(|_, _| Ok(None));
                store
                    .btc_address
                    .expect_insert()
                    .withf(|_, address, _| address == "bc1qfresh")
                    .times(1)
                    .returning(|wallet_id, address, _| Ok(btc_address(wallet_id, address)));

                let mut wallet = MockBitcoinWallet::new();
                wallet
                    .expect_new_address()
                    .times(1)
                    .returning(|_| Ok("bc1qfresh".to_string()));

                let service = service(store, wallet, MockEventUseCases::new(), MockSystemUseCases::new());

                let address = service.new_deposit_address(wallet_id, None).await.unwrap();

                assert_eq!(address.address, "bc1qfresh");
            }
        }
    }

    mod get_address {
        use super::*;

        mod when_missing {
            use super::*;

            #[tokio::test]
            async fn returns_not_found() {
                let mut store = MockAppStoreBuilder::new();
                store.btc_address.expect_find().times(1).returning(|_| Ok(None));

                let service = service(
                    store,
                    MockBitcoinWallet::new(),
                    MockEventUseCases::new(),
                    MockSystemUseCases::new(),
                );

                let err = service.get_address(Uuid::new_v4()).await.unwrap_err();

                assert!(matches!(err, ApplicationError::Data(DataError::NotFound(_))));
            }
        }
    }

    mod delete_address {
        use super::*;

        mod when_nothing_is_removed {
            use super::*;

            #[tokio::test]
            async fn returns_not_found() {
                let mut store = MockAppStoreBuilder::new();
                store.btc_address.expect_delete_many().times(1).returning(|_| Ok(0));

                let service = service(
                    store,
                    MockBitcoinWallet::new(),
                    MockEventUseCases::new(),
                    MockSystemUseCases::new(),
                );

                let err = service.delete_address(Uuid::new_v4()).await.unwrap_err();

                assert!(matches!(err, ApplicationError::Data(DataError::NotFound(_))));
            }
        }
    }

    mod sync {
        use super::*;

        mod with_an_existing_cursor_and_mixed_events {
            use super::*;

            #[tokio::test]
            async fn projects_each_event_and_advances_the_cursor() {
                let mut system = MockSystemUseCases::new();
                system
                    .expect_get_onchain_cursor()
                    .times(1)
                    .returning(|| Ok(Some(OnchainSyncCursor::BlockHeight(100))));
                system
                    .expect_set_onchain_cursor()
                    .withf(|cursor| *cursor == OnchainSyncCursor::BlockHeight(200))
                    .times(1)
                    .returning(|_| Ok(()));

                let mut wallet = MockBitcoinWallet::new();
                wallet.expect_network().returning(|| BtcNetwork::Regtest);
                wallet.expect_synchronize().times(1).returning(|_| {
                    Ok(OnchainSyncBatch {
                        events: vec![
                            OnchainTransaction::Deposit(BtcOutput {
                                amount_sat: 1_000,
                                ..Default::default()
                            }),
                            OnchainTransaction::Withdrawal(OnchainWithdrawalEvent {
                                txid: "txid".to_string(),
                                block_height: Some(200),
                            }),
                        ],
                        next_cursor: Some(OnchainSyncCursor::BlockHeight(200)),
                    })
                });

                let mut events = MockEventUseCases::new();
                events.expect_onchain_deposit().times(1).returning(|_| Ok(true));
                events.expect_onchain_withdrawal().times(1).returning(|_| Ok(true));

                let service = service(MockAppStoreBuilder::new(), wallet, events, system);

                assert_eq!(service.sync().await.unwrap(), 2);
            }
        }

        mod without_a_cursor {
            use super::*;

            #[tokio::test]
            async fn seeds_the_start_height_from_stored_data() {
                let mut system = MockSystemUseCases::new();
                system.expect_get_onchain_cursor().times(1).returning(|| Ok(None));
                // No next cursor is returned, so set_onchain_cursor must not be called.

                let mut store = MockAppStoreBuilder::new();
                store
                    .btc_output
                    .expect_max_block_height()
                    .times(1)
                    .returning(|| Ok(Some(50)));
                store
                    .payment
                    .expect_max_btc_block_height()
                    .times(1)
                    .returning(|| Ok(None));

                let mut wallet = MockBitcoinWallet::new();
                wallet.expect_network().returning(|| BtcNetwork::Regtest);
                wallet
                    .expect_synchronize()
                    .withf(|cursor| *cursor == Some(OnchainSyncCursor::BlockHeight(50)))
                    .times(1)
                    .returning(|_| Ok(OnchainSyncBatch::default()));

                let service = service(store, wallet, MockEventUseCases::new(), system);

                assert_eq!(service.sync().await.unwrap(), 0);
            }
        }
    }
}
