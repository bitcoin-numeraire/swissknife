use crate::application::{
    entities::AppStore,
    errors::{ApplicationError, DataError},
};
use async_trait::async_trait;
use regex::Regex;
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
        let email_username_re = Regex::new(r"^[a-zA-Z0-9.!#$%&'*+/=?^_`{|}~-]+$")
            .expect("should not fail as a constant");
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

        let balance = self.store.wallet.get_balance(None, id).await?;

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
