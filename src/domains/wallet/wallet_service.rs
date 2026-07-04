use crate::application::{
    composition::AppStore,
    errors::{ApplicationError, DataError},
};
use async_trait::async_trait;
use tracing::{debug, info, trace};
use uuid::Uuid;

use super::{Balance, Contact, Wallet, WalletFilter, WalletOverview, WalletUseCases};

pub struct WalletService {
    store: AppStore,
}

impl WalletService {
    pub fn new(store: AppStore) -> Self {
        WalletService { store }
    }
}

#[async_trait]
impl WalletUseCases for WalletService {
    async fn create(&self, account_id: Uuid, asset_id: Uuid) -> Result<Wallet, ApplicationError> {
        debug!(%account_id, %asset_id, "Creating account asset wallet");

        let wallet = self.store.wallet.ensure_for_account_asset(account_id, asset_id).await?;

        info!(id = %wallet.id, %account_id, %asset_id, "Wallet created or already existed");
        Ok(wallet)
    }

    async fn get(&self, id: Uuid) -> Result<Wallet, ApplicationError> {
        trace!(%id, "Fetching wallet");

        let wallet = self
            .store
            .wallet
            .find(id)
            .await?
            .ok_or_else(|| DataError::NotFound("Wallet not found.".to_string()))?;

        debug!(%id, "wallet fetched successfully");
        Ok(wallet)
    }

    async fn list(&self, filter: WalletFilter) -> Result<Vec<Wallet>, ApplicationError> {
        trace!(?filter, "Listing wallets");

        let wallets = self.store.wallet.find_many(filter.clone()).await?;

        debug!(?filter, "Wallets listed successfully");
        Ok(wallets)
    }

    async fn list_overviews(&self) -> Result<Vec<WalletOverview>, ApplicationError> {
        trace!("Listing wallet overviews");

        let overviews = self.store.wallet.find_many_overview().await?;

        debug!("Wallet overviews listed successfully");
        Ok(overviews)
    }

    async fn get_balance(&self, id: Uuid) -> Result<Balance, ApplicationError> {
        trace!(%id, "Fetching balance");

        let balance = self.store.wallet.get_balance(id).await?;

        debug!(%id, "Balance fetched successfully");
        Ok(balance)
    }

    async fn list_contacts(&self, id: Uuid) -> Result<Vec<Contact>, ApplicationError> {
        trace!(%id, "Fetching contacts");

        let contacts = self.store.wallet.find_contacts(id).await?;

        debug!(%id, "Contacts fetched successfully");
        Ok(contacts)
    }

    async fn delete(&self, id: Uuid) -> Result<(), ApplicationError> {
        debug!(%id, "Deleting wallet");

        let n_deleted = self
            .store
            .wallet
            .delete_many(WalletFilter {
                ids: Some(vec![id]),
                ..Default::default()
            })
            .await?;

        if n_deleted == 0 {
            return Err(DataError::NotFound("Wallet not found.".to_string()).into());
        }

        info!(%id, "Wallet deleted successfully");
        Ok(())
    }

    async fn delete_many(&self, filter: WalletFilter) -> Result<u64, ApplicationError> {
        debug!(?filter, "Deleting wallets");

        let n_deleted = self.store.wallet.delete_many(filter.clone()).await?;

        info!(?filter, n_deleted, "Wallets deleted successfully");
        Ok(n_deleted)
    }
}

#[cfg(test)]
mod tests {
    use crate::application::{composition::MockAppStoreBuilder, errors::DatabaseError};

    use super::*;

    fn wallet_fixture(id: Uuid, account_id: Uuid, asset_id: Uuid) -> Wallet {
        Wallet {
            id,
            account_id,
            asset_id,
            ..Default::default()
        }
    }

    mod create {
        use super::*;

        #[tokio::test]
        async fn ensures_and_returns_the_account_asset_wallet() {
            let account_id = Uuid::new_v4();
            let asset_id = Uuid::new_v4();
            let wallet_id = Uuid::new_v4();

            let mut store = MockAppStoreBuilder::new();
            store
                .wallet
                .expect_ensure_for_account_asset()
                .withf(move |account, asset| *account == account_id && *asset == asset_id)
                .times(1)
                .returning(move |account, asset| Ok(wallet_fixture(wallet_id, account, asset)));

            let service = WalletService::new(store.build());

            let wallet = service.create(account_id, asset_id).await.unwrap();

            assert_eq!(wallet.id, wallet_id);
            assert_eq!(wallet.account_id, account_id);
            assert_eq!(wallet.asset_id, asset_id);
        }
    }

    mod get {
        use super::*;

        mod when_found {
            use super::*;

            #[tokio::test]
            async fn returns_wallet() {
                let id = Uuid::new_v4();

                let mut store = MockAppStoreBuilder::new();
                store
                    .wallet
                    .expect_find()
                    .withf(move |queried| *queried == id)
                    .times(1)
                    .returning(|id| Ok(Some(wallet_fixture(id, Uuid::new_v4(), Uuid::new_v4()))));

                let service = WalletService::new(store.build());

                assert_eq!(service.get(id).await.unwrap().id, id);
            }
        }

        mod when_missing {
            use super::*;

            #[tokio::test]
            async fn returns_not_found() {
                let mut store = MockAppStoreBuilder::new();
                store.wallet.expect_find().times(1).returning(|_| Ok(None));

                let service = WalletService::new(store.build());

                let err = service.get(Uuid::new_v4()).await.unwrap_err();

                assert!(matches!(err, ApplicationError::Data(DataError::NotFound(_))));
            }
        }
    }

    mod get_balance {
        use super::*;

        #[tokio::test]
        async fn returns_balance_from_the_repository() {
            let id = Uuid::new_v4();

            let mut store = MockAppStoreBuilder::new();
            store
                .wallet
                .expect_get_balance()
                .withf(move |queried| *queried == id)
                .times(1)
                .returning(|_| {
                    Ok(Balance {
                        available_msat: 5_000,
                        ..Default::default()
                    })
                });

            let service = WalletService::new(store.build());

            assert_eq!(service.get_balance(id).await.unwrap().available_msat, 5_000);
        }

        #[tokio::test]
        async fn propagates_database_error() {
            let mut store = MockAppStoreBuilder::new();
            store
                .wallet
                .expect_get_balance()
                .times(1)
                .returning(|_| Err(DatabaseError::FindOne("boom".to_string())));

            let service = WalletService::new(store.build());

            let err = service.get_balance(Uuid::new_v4()).await.unwrap_err();

            assert!(matches!(err, ApplicationError::Database(DatabaseError::FindOne(_))));
        }
    }

    mod list_overviews {
        use super::*;

        #[tokio::test]
        async fn returns_overviews() {
            let mut store = MockAppStoreBuilder::new();
            store
                .wallet
                .expect_find_many_overview()
                .times(1)
                .returning(|| Ok(vec![WalletOverview::default()]));

            let service = WalletService::new(store.build());

            assert_eq!(service.list_overviews().await.unwrap().len(), 1);
        }
    }

    mod list_contacts {
        use super::*;

        #[tokio::test]
        async fn returns_contacts() {
            let mut store = MockAppStoreBuilder::new();
            store
                .wallet
                .expect_find_contacts()
                .times(1)
                .returning(|_| Ok(vec![Contact::default()]));

            let service = WalletService::new(store.build());

            assert_eq!(service.list_contacts(Uuid::new_v4()).await.unwrap().len(), 1);
        }
    }

    mod delete {
        use super::*;

        mod when_a_row_is_removed {
            use super::*;

            #[tokio::test]
            async fn succeeds() {
                let mut store = MockAppStoreBuilder::new();
                store.wallet.expect_delete_many().times(1).returning(|_| Ok(1));

                let service = WalletService::new(store.build());

                assert!(service.delete(Uuid::new_v4()).await.is_ok());
            }
        }

        mod when_nothing_is_removed {
            use super::*;

            #[tokio::test]
            async fn returns_not_found() {
                let mut store = MockAppStoreBuilder::new();
                store.wallet.expect_delete_many().times(1).returning(|_| Ok(0));

                let service = WalletService::new(store.build());

                let err = service.delete(Uuid::new_v4()).await.unwrap_err();

                assert!(matches!(err, ApplicationError::Data(DataError::NotFound(_))));
            }
        }
    }
}
