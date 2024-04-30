use async_trait::async_trait;
use tracing::{debug, info};

use crate::{
    application::errors::ApplicationError,
    domains::{
        lightning::{entities::UserBalance, usecases::WalletUseCases},
        users::entities::AuthUser,
    },
};

use super::LightningService;

#[async_trait]
impl WalletUseCases for LightningService {
    async fn get_balance(&self, user: AuthUser) -> Result<UserBalance, ApplicationError> {
        debug!(user_id = user.sub, "Fetching balance");

        let balance = self.store.get_balance(None, &user.sub).await?;

        info!(user_id = user.sub, "Balance fetched successfully");
        Ok(balance)
    }
}
