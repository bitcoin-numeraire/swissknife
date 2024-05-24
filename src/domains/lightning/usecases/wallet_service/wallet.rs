use async_trait::async_trait;
use tracing::{debug, trace};

use crate::{
    application::errors::ApplicationError,
    domains::{
        lightning::{entities::UserBalance, usecases::WalletUseCases},
        users::entities::AuthUser,
    },
};

use super::WalletService;

#[async_trait]
impl WalletUseCases for WalletService {
    async fn get_balance(&self, user: AuthUser) -> Result<UserBalance, ApplicationError> {
        trace!(user_id = user.sub, "Fetching balance");

        let balance = self.store.get_balance(None, &user.sub).await?;

        debug!(user_id = user.sub, "Balance fetched successfully");
        Ok(balance)
    }
}
