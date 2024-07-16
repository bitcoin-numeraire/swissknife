use crate::application::{
    entities::AppStore,
    errors::{ApplicationError, DataError},
};
use async_trait::async_trait;
use tracing::{debug, trace};
use uuid::Uuid;

use super::{Balance, Contact, Wallet, WalletUseCases};

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
    async fn get_balance(&self, id: Uuid) -> Result<Balance, ApplicationError> {
        trace!(%id, "Fetching balance");

        let balance = self.store.wallet.get_balance(None, id).await?;

        debug!(%id, "Balance fetched successfully");
        Ok(balance)
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

    async fn list_contacts(&self, id: Uuid) -> Result<Vec<Contact>, ApplicationError> {
        trace!(%id, "Fetching contacts");

        let contacts = self.store.wallet.find_contacts(id).await?;

        debug!(%id, "Contacts fetched successfully");
        Ok(contacts)
    }
}
