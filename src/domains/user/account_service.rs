use async_trait::async_trait;
use serde_json::Value;
use tracing::{debug, info, trace};
use uuid::Uuid;

use crate::application::{
    composition::AppStore,
    errors::{ApplicationError, DataError},
};

use super::{Account, AccountPreferences, AccountUseCases};

pub struct AccountService {
    store: AppStore,
}

impl AccountService {
    pub fn new(store: AppStore) -> Self {
        Self { store }
    }
}

#[async_trait]
impl AccountUseCases for AccountService {
    async fn get(&self, id: Uuid) -> Result<Account, ApplicationError> {
        trace!(%id, "Fetching account");

        let account = self
            .store
            .account
            .find(id)
            .await?
            .ok_or_else(|| DataError::NotFound("Account not found.".to_string()))?;

        debug!(%id, "Account fetched successfully");
        Ok(account)
    }

    async fn update_preferences(
        &self,
        id: Uuid,
        dashboard_settings: Value,
    ) -> Result<AccountPreferences, ApplicationError> {
        debug!(%id, "Updating account preferences");

        let preferences = self
            .store
            .account
            .update_preferences(id, dashboard_settings)
            .await?
            .ok_or_else(|| DataError::NotFound("Account preferences not found.".to_string()))?;

        info!(%id, "Account preferences updated successfully");
        Ok(preferences)
    }
}
