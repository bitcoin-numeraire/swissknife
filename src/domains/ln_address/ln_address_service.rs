use async_trait::async_trait;
use nostr_sdk::PublicKey;
use regex::Regex;
use tracing::{debug, info, trace};
use uuid::Uuid;

use crate::{
    application::{
        entities::AppStore,
        errors::{ApplicationError, DataError},
    },
    domains::ln_address::entities::{LnAddress, LnAddressFilter},
};

use super::LnAddressUseCases;

const MIN_USERNAME_LENGTH: usize = 1;
const MAX_USERNAME_LENGTH: usize = 64;

pub struct LnAddressService {
    store: AppStore,
}

impl LnAddressService {
    pub fn new(store: AppStore) -> Self {
        LnAddressService { store }
    }
}

#[async_trait]
impl LnAddressUseCases for LnAddressService {
    async fn register(
        &self,
        wallet_id: Uuid,
        mut username: String,
        allows_nostr: bool,
        nostr_pubkey: Option<PublicKey>,
    ) -> Result<LnAddress, ApplicationError> {
        debug!(%wallet_id, username, "Registering lightning address");

        username = username.to_lowercase();

        if username.len() < MIN_USERNAME_LENGTH || username.len() > MAX_USERNAME_LENGTH {
            return Err(DataError::Validation("Invalid username length.".to_string()).into());
        }

        // Regex validation for allowed characters in username
        let email_username_re =
            Regex::new(r"^[a-z0-9.!#$%&'*+/=?^_`{|}~-]+$").expect("should not fail as a constant");
        if !email_username_re.is_match(&username) {
            return Err(DataError::Validation("Invalid username format.".to_string()).into());
        }

        if self
            .store
            .ln_address
            .find_by_wallet_id(wallet_id)
            .await?
            .is_some()
        {
            return Err(DataError::Conflict("Duplicate User ID.".to_string()).into());
        }

        if self
            .store
            .ln_address
            .find_by_username(&username)
            .await?
            .is_some()
        {
            return Err(DataError::Conflict("Duplicate username.".to_string()).into());
        }

        let ln_address = self
            .store
            .ln_address
            .insert(wallet_id, &username, allows_nostr, nostr_pubkey)
            .await?;

        info!(
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

        info!(
            ?filter,
            n_deleted, "Lightning addresses deleted successfully"
        );
        Ok(n_deleted)
    }
}
