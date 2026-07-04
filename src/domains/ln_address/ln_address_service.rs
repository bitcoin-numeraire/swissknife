use async_trait::async_trait;
use nostr_sdk::PublicKey;
use regex::Regex;
use tracing::{debug, info, trace};
use uuid::Uuid;

use swissknife_types::UpdateLnAddressRequest;

use crate::{
    application::{
        composition::AppStore,
        errors::{ApplicationError, DataError},
    },
    domains::{
        bitcoin::BtcNetwork,
        ln_address::{LnAddress, LnAddressFilter},
    },
};

use super::LnAddressUseCases;

const MIN_USERNAME_LENGTH: usize = 1;
const MAX_USERNAME_LENGTH: usize = 64;

pub struct LnAddressService {
    store: AppStore,
    network: BtcNetwork,
}

impl LnAddressService {
    pub fn new(store: AppStore, network: BtcNetwork) -> Self {
        LnAddressService { store, network }
    }
}

#[async_trait]
impl LnAddressUseCases for LnAddressService {
    async fn register(
        &self,
        account_id: Uuid,
        mut username: String,
        allows_nostr: bool,
        nostr_pubkey: Option<PublicKey>,
    ) -> Result<LnAddress, ApplicationError> {
        debug!(%account_id, username, network = %self.network, "Registering lightning address");

        username = username.to_lowercase();
        validate_username(username.as_str())?;

        if self.store.ln_address.find_by_account_id(account_id).await?.is_some() {
            return Err(DataError::Conflict("Account already has a lightning address.".to_string()).into());
        }

        if self.store.ln_address.find_by_username(&username).await?.is_some() {
            return Err(DataError::Conflict("Duplicate username.".to_string()).into());
        }

        let asset = self
            .store
            .asset
            .find_native_btc_by_network(self.network)
            .await?
            .ok_or_else(|| DataError::Inconsistency("Native BTC asset is not configured.".to_string()))?;
        let wallet = self
            .store
            .wallet
            .find_by_account_and_asset(account_id, asset.id)
            .await?
            .ok_or_else(|| {
                DataError::Validation("Account has no native BTC wallet for the active network.".to_string())
            })?;
        let wallet_id = wallet.id;

        let ln_address = self
            .store
            .ln_address
            .insert(account_id, wallet_id, &username, allows_nostr, nostr_pubkey)
            .await?;

        info!(
            %account_id,
            %wallet_id,
            username, "Lightning address registered successfully"
        );
        Ok(ln_address)
    }

    async fn get(&self, id: Uuid) -> Result<LnAddress, ApplicationError> {
        trace!(%id, "Fetching lightning address");

        let ln_address = self
            .store
            .ln_address
            .find(id)
            .await?
            .ok_or_else(|| DataError::NotFound("Lightning address not found.".to_string()))?;

        debug!(
            %id, "Lightning address fetched successfully"
        );
        Ok(ln_address)
    }

    async fn list(&self, filter: LnAddressFilter) -> Result<Vec<LnAddress>, ApplicationError> {
        trace!(?filter, "Listing lightning addresses");

        let ln_addresses = self.store.ln_address.find_many(filter.clone()).await?;

        debug!(?filter, "Lightning addresses listed successfully");
        Ok(ln_addresses)
    }

    async fn update(&self, id: Uuid, request: UpdateLnAddressRequest) -> Result<LnAddress, ApplicationError> {
        debug!(%id, ?request, "Updating lightning address");

        let mut ln_address = self
            .store
            .ln_address
            .find(id)
            .await?
            .ok_or_else(|| DataError::NotFound("Lightning address not found.".to_string()))?;

        if let Some(mut username) = request.username {
            username = username.to_lowercase();

            if username != ln_address.username {
                validate_username(username.as_str())?;

                if self.store.ln_address.find_by_username(&username).await?.is_some() {
                    return Err(DataError::Conflict("Duplicate username.".to_string()).into());
                }

                ln_address.username = username;
            }
        }

        if let Some(active) = request.active {
            ln_address.active = active;
        }

        if let Some(allows_nostr) = request.allows_nostr {
            ln_address.allows_nostr = allows_nostr;
        }

        if let Some(nostr_pubkey) = request.nostr_pubkey {
            ln_address.nostr_pubkey = Some(nostr_pubkey);
        }

        let ln_address = self.store.ln_address.update(ln_address).await?;

        info!(%id, "Lightning address updated successfully");
        Ok(ln_address)
    }

    async fn delete(&self, id: Uuid) -> Result<(), ApplicationError> {
        debug!(%id, "Deleting lightning address");

        let n_deleted = self
            .store
            .ln_address
            .delete_many(LnAddressFilter {
                ids: Some(vec![id]),
                ..Default::default()
            })
            .await?;

        if n_deleted == 0 {
            return Err(DataError::NotFound("Lightning address not found.".to_string()).into());
        }

        info!(%id, "Lightning address deleted successfully");
        Ok(())
    }

    async fn delete_many(&self, filter: LnAddressFilter) -> Result<u64, ApplicationError> {
        debug!(?filter, "Deleting lightning addresses");

        let n_deleted = self.store.ln_address.delete_many(filter.clone()).await?;

        info!(?filter, n_deleted, "Lightning addresses deleted successfully");
        Ok(n_deleted)
    }
}

fn validate_username(username: &str) -> Result<(), DataError> {
    if username.len() < MIN_USERNAME_LENGTH || username.len() > MAX_USERNAME_LENGTH {
        return Err(DataError::Validation("Invalid username length.".to_string()));
    }

    // Regex validation for allowed characters in username
    let email_username_re = Regex::new(r"^[a-z0-9.!#$%&'*+/=?^_`{|}~-]+$").expect("should not fail as a constant");
    if !email_username_re.is_match(username) {
        return Err(DataError::Validation("Invalid username format.".to_string()));
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use chrono::Utc;

    use crate::{
        application::{composition::MockAppStoreBuilder, errors::DatabaseError},
        domains::{
            asset::{Asset, Protocol},
            bitcoin::BtcNetwork,
            wallet::Wallet,
        },
    };

    use super::*;

    const NATIVE_ASSET_REF: &str = "native";

    fn native_btc_asset() -> Asset {
        Asset {
            id: Uuid::new_v4(),
            code: "BTC".to_string(),
            name: Some("Bitcoin".to_string()),
            protocol: Protocol::Bitcoin,
            network: BtcNetwork::Regtest,
            asset_ref: NATIVE_ASSET_REF.to_string(),
            display_ticker: "rBTC".to_string(),
            decimals: 11,
            created_at: Utc::now(),
            updated_at: None,
        }
    }

    fn wallet(id: Uuid, account_id: Uuid) -> Wallet {
        let asset = native_btc_asset();
        Wallet {
            id,
            account_id,
            asset_id: asset.id,
            asset: Some(asset),
            ..Default::default()
        }
    }

    fn ln_address_fixture(id: Uuid, account_id: Uuid, wallet_id: Uuid, username: &str) -> LnAddress {
        LnAddress {
            id,
            account_id,
            wallet_id,
            username: username.to_string(),
            active: true,
            allows_nostr: false,
            nostr_pubkey: None,
            created_at: Utc::now(),
            updated_at: None,
        }
    }

    fn update_request(username: Option<&str>) -> UpdateLnAddressRequest {
        UpdateLnAddressRequest {
            username: username.map(str::to_string),
            active: None,
            allows_nostr: None,
            nostr_pubkey: None,
        }
    }

    mod validate_username {
        use super::*;

        #[test]
        fn accepts_supported_email_local_part_characters() {
            assert!(validate_username("alice").is_ok());
            assert!(validate_username("alice.123_+-").is_ok());
            assert!(validate_username("a".repeat(MAX_USERNAME_LENGTH).as_str()).is_ok());
        }

        #[test]
        fn rejects_empty_or_too_long_usernames() {
            let empty_err = validate_username("").unwrap_err();
            assert!(matches!(empty_err, DataError::Validation(_)));
            assert!(empty_err.to_string().contains("Invalid username length"));

            let too_long = "a".repeat(MAX_USERNAME_LENGTH + 1);
            let too_long_err = validate_username(&too_long).unwrap_err();
            assert!(matches!(too_long_err, DataError::Validation(_)));
            assert!(too_long_err.to_string().contains("Invalid username length"));
        }

        #[test]
        fn rejects_unsupported_characters() {
            for username in ["Alice", "alice bob", "alice@example", "alice:123"] {
                let err = validate_username(username).unwrap_err();
                assert!(matches!(err, DataError::Validation(_)));
                assert!(err.to_string().contains("Invalid username format"));
            }
        }
    }

    mod register {
        use super::*;

        mod with_valid_new_username {
            use super::*;

            #[tokio::test]
            async fn lowercases_username_and_inserts() {
                let account_id = Uuid::new_v4();
                let wallet_id = Uuid::new_v4();
                let asset = native_btc_asset();
                let asset_id = asset.id;

                let mut store = MockAppStoreBuilder::new();
                store
                    .ln_address
                    .expect_find_by_account_id()
                    .withf(move |id| *id == account_id)
                    .times(1)
                    .returning(|_| Ok(None));
                store
                    .ln_address
                    .expect_find_by_username()
                    .withf(|username| username == "alice")
                    .times(1)
                    .returning(|_| Ok(None));
                store
                    .asset
                    .expect_find_native_btc_by_network()
                    .withf(|network| *network == BtcNetwork::Regtest)
                    .times(1)
                    .returning(move |_| Ok(Some(asset.clone())));
                store
                    .wallet
                    .expect_find_by_account_and_asset()
                    .withf(move |account, asset| *account == account_id && *asset == asset_id)
                    .times(1)
                    .returning(move |account, _| Ok(Some(wallet(wallet_id, account))));
                store
                    .ln_address
                    .expect_insert()
                    .withf(move |account, wallet, username, _, _| {
                        *account == account_id && *wallet == wallet_id && username == "alice"
                    })
                    .times(1)
                    .returning(|account_id, wallet_id, username, _, _| {
                        Ok(ln_address_fixture(Uuid::new_v4(), account_id, wallet_id, username))
                    });

                let service = LnAddressService::new(store.build(), BtcNetwork::Regtest);

                let ln_address = service
                    .register(account_id, "Alice".to_string(), false, None)
                    .await
                    .unwrap();

                assert_eq!(ln_address.username, "alice");
                assert_eq!(ln_address.account_id, account_id);
                assert_eq!(ln_address.wallet_id, wallet_id);
            }
        }

        mod with_invalid_username {
            use super::*;

            #[tokio::test]
            async fn rejects_without_touching_the_store() {
                // No store expectations are installed, so any repository call panics.
                let service = LnAddressService::new(MockAppStoreBuilder::new().build(), BtcNetwork::Regtest);

                let err = service
                    .register(Uuid::new_v4(), "invalid username".to_string(), false, None)
                    .await
                    .unwrap_err();

                assert!(matches!(err, ApplicationError::Data(DataError::Validation(_))));
            }
        }

        mod when_account_already_has_an_address {
            use super::*;

            #[tokio::test]
            async fn returns_conflict() {
                let account_id = Uuid::new_v4();
                let wallet_id = Uuid::new_v4();

                let mut store = MockAppStoreBuilder::new();
                store
                    .ln_address
                    .expect_find_by_account_id()
                    .times(1)
                    .returning(move |_| {
                        Ok(Some(ln_address_fixture(
                            Uuid::new_v4(),
                            account_id,
                            wallet_id,
                            "existing",
                        )))
                    });

                let service = LnAddressService::new(store.build(), BtcNetwork::Regtest);

                let err = service
                    .register(account_id, "alice".to_string(), false, None)
                    .await
                    .unwrap_err();

                assert!(matches!(err, ApplicationError::Data(DataError::Conflict(_))));
                assert!(err.to_string().contains("Account already has a lightning address"));
            }
        }

        mod when_username_is_taken {
            use super::*;

            #[tokio::test]
            async fn returns_conflict() {
                let mut store = MockAppStoreBuilder::new();
                store
                    .ln_address
                    .expect_find_by_account_id()
                    .times(1)
                    .returning(|_| Ok(None));
                store.ln_address.expect_find_by_username().times(1).returning(|_| {
                    Ok(Some(ln_address_fixture(
                        Uuid::new_v4(),
                        Uuid::new_v4(),
                        Uuid::new_v4(),
                        "alice",
                    )))
                });

                let service = LnAddressService::new(store.build(), BtcNetwork::Regtest);

                let err = service
                    .register(Uuid::new_v4(), "alice".to_string(), false, None)
                    .await
                    .unwrap_err();

                assert!(matches!(err, ApplicationError::Data(DataError::Conflict(_))));
                assert!(err.to_string().contains("Duplicate username"));
            }
        }

        mod when_lookup_fails {
            use super::*;

            #[tokio::test]
            async fn propagates_database_error() {
                let mut store = MockAppStoreBuilder::new();
                store
                    .ln_address
                    .expect_find_by_account_id()
                    .times(1)
                    .returning(|_| Err(DatabaseError::FindOne("boom".to_string())));

                let service = LnAddressService::new(store.build(), BtcNetwork::Regtest);

                let err = service
                    .register(Uuid::new_v4(), "alice".to_string(), false, None)
                    .await
                    .unwrap_err();

                assert!(matches!(err, ApplicationError::Database(DatabaseError::FindOne(_))));
            }
        }

        mod when_active_network_asset_is_missing {
            use super::*;

            #[tokio::test]
            async fn returns_inconsistency() {
                let mut store = MockAppStoreBuilder::new();
                store
                    .ln_address
                    .expect_find_by_account_id()
                    .times(1)
                    .returning(|_| Ok(None));
                store
                    .ln_address
                    .expect_find_by_username()
                    .times(1)
                    .returning(|_| Ok(None));
                store
                    .asset
                    .expect_find_native_btc_by_network()
                    .withf(|network| *network == BtcNetwork::Regtest)
                    .times(1)
                    .returning(|_| Ok(None));

                let service = LnAddressService::new(store.build(), BtcNetwork::Regtest);

                let err = service
                    .register(Uuid::new_v4(), "alice".to_string(), false, None)
                    .await
                    .unwrap_err();

                assert!(matches!(err, ApplicationError::Data(DataError::Inconsistency(_))));
                assert!(err.to_string().contains("Native BTC asset is not configured"));
            }
        }

        mod when_account_has_no_active_network_wallet {
            use super::*;

            #[tokio::test]
            async fn returns_validation_error() {
                let account_id = Uuid::new_v4();
                let asset = native_btc_asset();
                let asset_id = asset.id;

                let mut store = MockAppStoreBuilder::new();
                store
                    .ln_address
                    .expect_find_by_account_id()
                    .times(1)
                    .returning(|_| Ok(None));
                store
                    .ln_address
                    .expect_find_by_username()
                    .times(1)
                    .returning(|_| Ok(None));
                store
                    .asset
                    .expect_find_native_btc_by_network()
                    .withf(|network| *network == BtcNetwork::Regtest)
                    .times(1)
                    .returning(move |_| Ok(Some(asset.clone())));
                store
                    .wallet
                    .expect_find_by_account_and_asset()
                    .withf(move |account, asset| *account == account_id && *asset == asset_id)
                    .times(1)
                    .returning(|_, _| Ok(None));

                let service = LnAddressService::new(store.build(), BtcNetwork::Regtest);

                let err = service
                    .register(account_id, "alice".to_string(), false, None)
                    .await
                    .unwrap_err();

                assert!(matches!(err, ApplicationError::Data(DataError::Validation(_))));
                assert!(err.to_string().contains("native BTC wallet"));
            }
        }
    }

    mod get {
        use super::*;

        mod when_found {
            use super::*;

            #[tokio::test]
            async fn returns_address() {
                let id = Uuid::new_v4();

                let mut store = MockAppStoreBuilder::new();
                store
                    .ln_address
                    .expect_find()
                    .withf(move |queried| *queried == id)
                    .times(1)
                    .returning(move |id| Ok(Some(ln_address_fixture(id, Uuid::new_v4(), Uuid::new_v4(), "alice"))));

                let service = LnAddressService::new(store.build(), BtcNetwork::Regtest);

                let ln_address = service.get(id).await.unwrap();

                assert_eq!(ln_address.id, id);
            }
        }

        mod when_missing {
            use super::*;

            #[tokio::test]
            async fn returns_not_found() {
                let mut store = MockAppStoreBuilder::new();
                store.ln_address.expect_find().times(1).returning(|_| Ok(None));

                let service = LnAddressService::new(store.build(), BtcNetwork::Regtest);

                let err = service.get(Uuid::new_v4()).await.unwrap_err();

                assert!(matches!(err, ApplicationError::Data(DataError::NotFound(_))));
            }
        }
    }

    mod list {
        use super::*;

        #[tokio::test]
        async fn returns_addresses_from_the_repository() {
            let mut store = MockAppStoreBuilder::new();
            store.ln_address.expect_find_many().times(1).returning(|_| {
                Ok(vec![ln_address_fixture(
                    Uuid::new_v4(),
                    Uuid::new_v4(),
                    Uuid::new_v4(),
                    "alice",
                )])
            });

            let service = LnAddressService::new(store.build(), BtcNetwork::Regtest);

            let addresses = service.list(LnAddressFilter::default()).await.unwrap();

            assert_eq!(addresses.len(), 1);
        }
    }

    mod update {
        use super::*;

        mod when_address_is_missing {
            use super::*;

            #[tokio::test]
            async fn returns_not_found() {
                let mut store = MockAppStoreBuilder::new();
                store.ln_address.expect_find().times(1).returning(|_| Ok(None));

                let service = LnAddressService::new(store.build(), BtcNetwork::Regtest);

                let err = service
                    .update(Uuid::new_v4(), update_request(Some("bob")))
                    .await
                    .unwrap_err();

                assert!(matches!(err, ApplicationError::Data(DataError::NotFound(_))));
            }
        }

        mod with_a_new_unique_username {
            use super::*;

            #[tokio::test]
            async fn validates_uniqueness_and_persists() {
                let id = Uuid::new_v4();

                let mut store = MockAppStoreBuilder::new();
                store
                    .ln_address
                    .expect_find()
                    .times(1)
                    .returning(move |id| Ok(Some(ln_address_fixture(id, Uuid::new_v4(), Uuid::new_v4(), "alice"))));
                store
                    .ln_address
                    .expect_find_by_username()
                    .withf(|username| username == "bob")
                    .times(1)
                    .returning(|_| Ok(None));
                store
                    .ln_address
                    .expect_update()
                    .withf(|ln_address| ln_address.username == "bob")
                    .times(1)
                    .returning(Ok);

                let service = LnAddressService::new(store.build(), BtcNetwork::Regtest);

                let updated = service.update(id, update_request(Some("Bob"))).await.unwrap();

                assert_eq!(updated.username, "bob");
            }
        }

        mod when_username_is_unchanged {
            use super::*;

            #[tokio::test]
            async fn skips_uniqueness_check() {
                let mut store = MockAppStoreBuilder::new();
                store
                    .ln_address
                    .expect_find()
                    .times(1)
                    .returning(|id| Ok(Some(ln_address_fixture(id, Uuid::new_v4(), Uuid::new_v4(), "alice"))));
                // find_by_username is intentionally not expected: an unchanged
                // username must not trigger a uniqueness lookup.
                store.ln_address.expect_update().times(1).returning(Ok);

                let service = LnAddressService::new(store.build(), BtcNetwork::Regtest);

                let updated = service
                    .update(Uuid::new_v4(), update_request(Some("alice")))
                    .await
                    .unwrap();

                assert_eq!(updated.username, "alice");
            }
        }

        mod when_new_username_is_taken {
            use super::*;

            #[tokio::test]
            async fn returns_conflict() {
                let mut store = MockAppStoreBuilder::new();
                store
                    .ln_address
                    .expect_find()
                    .times(1)
                    .returning(|id| Ok(Some(ln_address_fixture(id, Uuid::new_v4(), Uuid::new_v4(), "alice"))));
                store.ln_address.expect_find_by_username().times(1).returning(|_| {
                    Ok(Some(ln_address_fixture(
                        Uuid::new_v4(),
                        Uuid::new_v4(),
                        Uuid::new_v4(),
                        "bob",
                    )))
                });

                let service = LnAddressService::new(store.build(), BtcNetwork::Regtest);

                let err = service
                    .update(Uuid::new_v4(), update_request(Some("bob")))
                    .await
                    .unwrap_err();

                assert!(matches!(err, ApplicationError::Data(DataError::Conflict(_))));
            }
        }
    }

    mod delete {
        use super::*;

        mod when_a_row_is_removed {
            use super::*;

            #[tokio::test]
            async fn succeeds() {
                let mut store = MockAppStoreBuilder::new();
                store.ln_address.expect_delete_many().times(1).returning(|_| Ok(1));

                let service = LnAddressService::new(store.build(), BtcNetwork::Regtest);

                assert!(service.delete(Uuid::new_v4()).await.is_ok());
            }
        }

        mod when_nothing_is_removed {
            use super::*;

            #[tokio::test]
            async fn returns_not_found() {
                let mut store = MockAppStoreBuilder::new();
                store.ln_address.expect_delete_many().times(1).returning(|_| Ok(0));

                let service = LnAddressService::new(store.build(), BtcNetwork::Regtest);

                let err = service.delete(Uuid::new_v4()).await.unwrap_err();

                assert!(matches!(err, ApplicationError::Data(DataError::NotFound(_))));
            }
        }
    }

    mod delete_many {
        use super::*;

        #[tokio::test]
        async fn returns_deleted_count() {
            let mut store = MockAppStoreBuilder::new();
            store.ln_address.expect_delete_many().times(1).returning(|_| Ok(3));

            let service = LnAddressService::new(store.build(), BtcNetwork::Regtest);

            let deleted = service.delete_many(LnAddressFilter::default()).await.unwrap();

            assert_eq!(deleted, 3);
        }
    }
}
