use async_trait::async_trait;

use crate::application::errors::ApplicationError;

use super::{Contact, UserBalance, Wallet};

#[async_trait]
pub trait WalletUseCases: Send + Sync {
    async fn get(&self, user_id: String) -> Result<Wallet, ApplicationError>;
    async fn get_balance(&self, user: String) -> Result<UserBalance, ApplicationError>;
    async fn list_contacts(&self, user_id: String) -> Result<Vec<Contact>, ApplicationError>;
}
