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

        if ln_address.allows_nostr && ln_address.nostr_pubkey.is_some() {
            debug!(username, "Nostr identifier fetched successfully");
            Ok(ln_address.nostr_pubkey.unwrap())
        } else {
            debug!(username, "Nostr identifier not enabled");
            Err(DataError::NotFound("Nostr ID not found.".to_string()).into())
        }
    }
}
