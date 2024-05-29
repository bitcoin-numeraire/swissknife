use async_trait::async_trait;
use regex::Regex;
use tracing::{debug, info, trace};
use uuid::Uuid;

use crate::{
    application::{
        entities::AppStore,
        errors::{ApplicationError, DataError},
    },
    domains::lightning::entities::{LnAddress, LnAddressFilter, LnURLPayRequest},
};

use super::LnAddressesUseCases;

const MIN_USERNAME_LENGTH: usize = 1;
const MAX_USERNAME_LENGTH: usize = 64;

pub struct LnAddressService {
    domain: String,
    store: AppStore,
}

impl LnAddressService {
    pub fn new(store: AppStore, domain: String) -> Self {
        LnAddressService { store, domain }
    }
}

#[async_trait]
impl LnAddressesUseCases for LnAddressService {
    async fn lnurlp(&self, username: String) -> Result<LnURLPayRequest, ApplicationError> {
        debug!(username, "Generating LNURLp");

        self.store
            .ln_address
            .find_by_username(&username)
            .await?
            .ok_or_else(|| DataError::NotFound("Lightning address not found.".to_string()))?;

        let metadata = serde_json::to_string(&[
            [
                "text/plain".to_string(),
                format!("{} never refuses sats", username),
            ],
            [
                "text/identifier".to_string(),
                format!("{}@{}", username, self.domain),
            ],
        ])
        .unwrap();

        let lnurlp = LnURLPayRequest {
            callback: format!("https://{}/api/lnurlp/{}/callback", self.domain, username),
            max_sendable: 1000000000,
            min_sendable: 1000,
            metadata,
            comment_allowed: 255,
            tag: "payRequest".to_string(),
        };

        info!(username, "LNURLp returned successfully");
        Ok(lnurlp)
    }

    async fn register(
        &self,
        user_id: String,
        username: String,
    ) -> Result<LnAddress, ApplicationError> {
        debug!(user_id, username, "Registering lightning address");

        if username.len() < MIN_USERNAME_LENGTH || username.len() > MAX_USERNAME_LENGTH {
            return Err(DataError::Validation("Invalid username length.".to_string()).into());
        }

        // Regex validation for allowed characters
        let email_username_re = Regex::new(r"^[a-zA-Z0-9.!#$%&'*+/=?^_`{|}~-]+$").unwrap(); // Can't fail by assertion
        if !email_username_re.is_match(&username) {
            return Err(DataError::Validation("Invalid username format.".to_string()).into());
        }

        if self
            .store
            .ln_address
            .find_by_user_id(&user_id)
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

        let ln_address = self.store.ln_address.insert(&user_id, &username).await?;

        info!(
            user_id,
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
                id: Some(id),
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
