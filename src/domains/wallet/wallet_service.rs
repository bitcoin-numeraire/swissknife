use crate::application::{
    composition::{AppStore, Currency},
    errors::{ApplicationError, DataError},
};
use async_trait::async_trait;
use regex::Regex;
use tracing::{debug, info, trace};
use uuid::Uuid;

use super::{Balance, Contact, Wallet, WalletFilter, WalletOverview, WalletUseCases};

pub struct WalletService {
    store: AppStore,
    currency: Currency,
}

impl WalletService {
    pub fn new(store: AppStore, currency: Currency) -> Self {
        WalletService { store, currency }
    }
}

const MIN_USER_LENGTH: usize = 1;
const MAX_USER_LENGTH: usize = 64;

#[async_trait]
impl WalletUseCases for WalletService {
    async fn register(&self, user_id: String) -> Result<Wallet, ApplicationError> {
        debug!(%user_id, "Registering wallet");

        if user_id.len() < MIN_USER_LENGTH || user_id.len() > MAX_USER_LENGTH {
            return Err(DataError::Validation("Invalid user_id length.".to_string()).into());
        }

        // Regex validation for allowed characters
        let email_username_re =
            Regex::new(r"^[a-zA-Z0-9.!#$%&'*+/=?^_`{|}~-]+$").expect("should not fail as a constant");
        if !email_username_re.is_match(&user_id) {
            return Err(DataError::Validation("Invalid user_id format.".to_string()).into());
        }

        if self.store.wallet.find_by_user_id(&user_id).await?.is_some() {
            return Err(DataError::Conflict("Duplicate User ID.".to_string()).into());
        }

        let wallet = self.store.wallet.insert(&user_id).await?;

        info!(id = %wallet.id, "Wallet registered successfully");
        Ok(wallet)
    }

    async fn get(&self, id: Uuid) -> Result<Wallet, ApplicationError> {
        trace!(%id, "Fetching wallet");

        let wallet = self
            .store
            .wallet
            .find(id, &self.currency)
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

        let overviews = self.store.wallet.find_many_overview(&self.currency).await?;

        debug!("Wallet overviews listed successfully");
        Ok(overviews)
    }

    async fn get_balance(&self, id: Uuid) -> Result<Balance, ApplicationError> {
        trace!(%id, "Fetching balance");

        let balance = self.store.wallet.get_balance(id, &self.currency).await?;

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

    fn wallet_fixture(id: Uuid, user_id: &str) -> Wallet {
        Wallet {
            id,
            user_id: user_id.to_string(),
            ..Default::default()
        }
    }

    mod register {
        use super::*;

        mod with_a_valid_new_user_id {
            use super::*;

            #[tokio::test]
            async fn inserts_and_returns_wallet() {
                let mut store = MockAppStoreBuilder::new();
                store
                    .wallet
                    .expect_find_by_user_id()
                    .withf(|user_id| user_id == "alice")
                    .times(1)
                    .returning(|_| Ok(None));
                store
                    .wallet
                    .expect_insert()
                    .withf(|user_id| user_id == "alice")
                    .times(1)
                    .returning(|user_id| Ok(wallet_fixture(Uuid::new_v4(), user_id)));

                let service = WalletService::new(store.build(), Currency::Regtest);

                let wallet = service.register("alice".to_string()).await.unwrap();

                assert_eq!(wallet.user_id, "alice");
            }
        }

        mod with_an_invalid_length {
            use super::*;

            #[tokio::test]
            async fn rejects_empty_user_id() {
                let service = WalletService::new(MockAppStoreBuilder::new().build(), Currency::Regtest);

                let err = service.register(String::new()).await.unwrap_err();

                assert!(matches!(err, ApplicationError::Data(DataError::Validation(_))));
            }

            #[tokio::test]
            async fn rejects_too_long_user_id() {
                let service = WalletService::new(MockAppStoreBuilder::new().build(), Currency::Regtest);

                let err = service.register("a".repeat(MAX_USER_LENGTH + 1)).await.unwrap_err();

                assert!(matches!(err, ApplicationError::Data(DataError::Validation(_))));
            }
        }

        mod with_an_invalid_format {
            use super::*;

            #[tokio::test]
            async fn rejects_disallowed_characters() {
                let service = WalletService::new(MockAppStoreBuilder::new().build(), Currency::Regtest);

                let err = service.register("alice bob".to_string()).await.unwrap_err();

                assert!(matches!(err, ApplicationError::Data(DataError::Validation(_))));
                assert!(err.to_string().contains("Invalid user_id format"));
            }
        }

        mod when_user_id_already_exists {
            use super::*;

            #[tokio::test]
            async fn returns_conflict() {
                let mut store = MockAppStoreBuilder::new();
                store
                    .wallet
                    .expect_find_by_user_id()
                    .times(1)
                    .returning(|user_id| Ok(Some(wallet_fixture(Uuid::new_v4(), user_id))));

                let service = WalletService::new(store.build(), Currency::Regtest);

                let err = service.register("alice".to_string()).await.unwrap_err();

                assert!(matches!(err, ApplicationError::Data(DataError::Conflict(_))));
            }
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
                    .withf(move |queried, _| *queried == id)
                    .times(1)
                    .returning(|id, _| Ok(Some(wallet_fixture(id, "alice"))));

                let service = WalletService::new(store.build(), Currency::Regtest);

                assert_eq!(service.get(id).await.unwrap().id, id);
            }
        }

        mod when_missing {
            use super::*;

            #[tokio::test]
            async fn returns_not_found() {
                let mut store = MockAppStoreBuilder::new();
                store.wallet.expect_find().times(1).returning(|_, _| Ok(None));

                let service = WalletService::new(store.build(), Currency::Regtest);

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
                .withf(move |queried, _| *queried == id)
                .times(1)
                .returning(|_, _| {
                    Ok(Balance {
                        available_msat: 5_000,
                        ..Default::default()
                    })
                });

            let service = WalletService::new(store.build(), Currency::Regtest);

            assert_eq!(service.get_balance(id).await.unwrap().available_msat, 5_000);
        }

        #[tokio::test]
        async fn propagates_database_error() {
            let mut store = MockAppStoreBuilder::new();
            store
                .wallet
                .expect_get_balance()
                .times(1)
                .returning(|_, _| Err(DatabaseError::FindOne("boom".to_string())));

            let service = WalletService::new(store.build(), Currency::Regtest);

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
                .returning(|_| Ok(vec![WalletOverview::default()]));

            let service = WalletService::new(store.build(), Currency::Regtest);

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

            let service = WalletService::new(store.build(), Currency::Regtest);

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

                let service = WalletService::new(store.build(), Currency::Regtest);

                assert!(service.delete(Uuid::new_v4()).await.is_ok());
            }
        }

        mod when_nothing_is_removed {
            use super::*;

            #[tokio::test]
            async fn returns_not_found() {
                let mut store = MockAppStoreBuilder::new();
                store.wallet.expect_delete_many().times(1).returning(|_| Ok(0));

                let service = WalletService::new(store.build(), Currency::Regtest);

                let err = service.delete(Uuid::new_v4()).await.unwrap_err();

                assert!(matches!(err, ApplicationError::Data(DataError::NotFound(_))));
            }
        }
    }
}
