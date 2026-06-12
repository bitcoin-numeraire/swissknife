use async_trait::async_trait;
use nostr_sdk::PublicKey;
use tracing::{debug, trace};

use crate::application::{
    composition::AppStore,
    errors::{ApplicationError, DataError},
};

use super::NostrUseCases;

pub struct NostrService {
    store: AppStore,
}

impl NostrService {
    pub fn new(store: AppStore) -> Self {
        NostrService { store }
    }
}

#[async_trait]
impl NostrUseCases for NostrService {
    async fn get_pubkey(&self, username: String) -> Result<PublicKey, ApplicationError> {
        trace!(username, "Fetching Nostr identifier");

        let ln_address = self
            .store
            .ln_address
            .find_by_username(&username)
            .await?
            .ok_or_else(|| DataError::NotFound("Nostr ID not found.".to_string()))?;

        if let (true, Some(nostr_pubkey)) = (ln_address.allows_nostr, ln_address.nostr_pubkey) {
            debug!(username, "Nostr identifier fetched successfully");
            Ok(nostr_pubkey)
        } else {
            debug!(username, "Nostr identifier not enabled");
            Err(DataError::NotFound("Nostr ID not found.".to_string()).into())
        }
    }
}

#[cfg(test)]
mod tests {
    use chrono::Utc;
    use uuid::Uuid;

    use crate::{
        application::{composition::MockAppStoreBuilder, errors::DatabaseError},
        domains::ln_address::LnAddress,
    };

    use super::*;

    // Generator point x-coordinate: a valid x-only (Schnorr) public key.
    const VALID_PUBKEY_HEX: &str = "79be667ef9dcbbac55a06295ce870b07029bfcdb2dce28d959f2815b16f81798";

    fn ln_address_fixture(allows_nostr: bool, nostr_pubkey: Option<PublicKey>) -> LnAddress {
        LnAddress {
            id: Uuid::new_v4(),
            wallet_id: Uuid::new_v4(),
            username: "alice".to_string(),
            active: true,
            allows_nostr,
            nostr_pubkey,
            created_at: Utc::now(),
            updated_at: None,
        }
    }

    mod get_pubkey {
        use super::*;

        mod when_nostr_is_enabled_with_a_pubkey {
            use super::*;

            #[tokio::test]
            async fn returns_the_pubkey() {
                let pubkey = PublicKey::from_hex(VALID_PUBKEY_HEX).unwrap();

                let mut store = MockAppStoreBuilder::new();
                store
                    .ln_address
                    .expect_find_by_username()
                    .withf(|username| username == "alice")
                    .times(1)
                    .returning(move |_| Ok(Some(ln_address_fixture(true, Some(pubkey)))));

                let service = NostrService::new(store.build());

                let result = service.get_pubkey("alice".to_string()).await.unwrap();

                assert_eq!(result, pubkey);
            }
        }

        mod when_address_is_missing {
            use super::*;

            #[tokio::test]
            async fn returns_not_found() {
                let mut store = MockAppStoreBuilder::new();
                store
                    .ln_address
                    .expect_find_by_username()
                    .times(1)
                    .returning(|_| Ok(None));

                let service = NostrService::new(store.build());

                let err = service.get_pubkey("alice".to_string()).await.unwrap_err();

                assert!(matches!(err, ApplicationError::Data(DataError::NotFound(_))));
            }
        }

        mod when_nostr_is_disabled {
            use super::*;

            #[tokio::test]
            async fn returns_not_found() {
                let pubkey = PublicKey::from_hex(VALID_PUBKEY_HEX).unwrap();

                let mut store = MockAppStoreBuilder::new();
                store
                    .ln_address
                    .expect_find_by_username()
                    .times(1)
                    .returning(move |_| Ok(Some(ln_address_fixture(false, Some(pubkey)))));

                let service = NostrService::new(store.build());

                let err = service.get_pubkey("alice".to_string()).await.unwrap_err();

                assert!(matches!(err, ApplicationError::Data(DataError::NotFound(_))));
            }
        }

        mod when_pubkey_is_absent {
            use super::*;

            #[tokio::test]
            async fn returns_not_found() {
                let mut store = MockAppStoreBuilder::new();
                store
                    .ln_address
                    .expect_find_by_username()
                    .times(1)
                    .returning(|_| Ok(Some(ln_address_fixture(true, None))));

                let service = NostrService::new(store.build());

                let err = service.get_pubkey("alice".to_string()).await.unwrap_err();

                assert!(matches!(err, ApplicationError::Data(DataError::NotFound(_))));
            }
        }

        mod when_lookup_fails {
            use super::*;

            #[tokio::test]
            async fn propagates_database_error() {
                let mut store = MockAppStoreBuilder::new();
                store
                    .ln_address
                    .expect_find_by_username()
                    .times(1)
                    .returning(|_| Err(DatabaseError::FindOne("boom".to_string())));

                let service = NostrService::new(store.build());

                let err = service.get_pubkey("alice".to_string()).await.unwrap_err();

                assert!(matches!(err, ApplicationError::Database(DatabaseError::FindOne(_))));
            }
        }
    }
}
