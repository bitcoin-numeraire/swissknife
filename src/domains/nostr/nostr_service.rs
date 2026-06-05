use async_trait::async_trait;
use nostr_sdk::PublicKey;
use tracing::{debug, trace};

use crate::application::{
    entities::AppStore,
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
    use std::str::FromStr;

    use chrono::Utc;
    use mockall::predicate::eq;
    use nostr_sdk::PublicKey;
    use uuid::Uuid;

    use crate::{
        application::entities::AppStore,
        domains::{
            ln_address::{LnAddress, MockLnAddressRepository},
            nostr::NostrUseCases,
        },
    };

    use super::NostrService;

    fn ln_address(username: &str, nostr_pubkey: Option<PublicKey>) -> LnAddress {
        LnAddress {
            id: Uuid::new_v4(),
            wallet_id: Uuid::new_v4(),
            username: username.to_string(),
            active: true,
            allows_nostr: nostr_pubkey.is_some(),
            nostr_pubkey,
            created_at: Utc::now(),
            updated_at: None,
        }
    }

    #[tokio::test]
    async fn get_pubkey_uses_injected_ln_address_repository() {
        let username = "satoshi";
        let pubkey = PublicKey::from_str("6e468422af5bc8a09f2b842b07d0ce8ba85e03d4cbd06bd348289409c0d5a7e7").unwrap();

        let mut store = AppStore::mock();
        store
            .ln_address
            .expect_find_by_username()
            .with(eq(username))
            .times(1)
            .returning(move |_| Ok(Some(ln_address(username, Some(pubkey)))));

        let service = NostrService::new(store.build());

        assert_eq!(service.get_pubkey(username.to_string()).await.unwrap(), pubkey);
    }

    #[test]
    fn app_store_mock_builder_accepts_generated_repository_mocks() {
        let mut ln_address = MockLnAddressRepository::new();
        ln_address.expect_find_by_username().never();

        let mut store = AppStore::mock();
        store.ln_address = ln_address;

        let _store = store.build();
    }
}
