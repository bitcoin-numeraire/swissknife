use async_trait::async_trait;

use crate::{
    application::errors::ApplicationError,
    domains::{lightning::entities::UserBalance, users::entities::AuthUser},
};

#[async_trait]
pub trait WalletUseCases: Send + Sync {
    async fn get_balance(&self, user: AuthUser) -> Result<UserBalance, ApplicationError>;
}
