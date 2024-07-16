use async_trait::async_trait;
use uuid::Uuid;

use crate::application::errors::ApplicationError;

use super::{Contact, UserBalance, Wallet};

#[async_trait]
pub trait WalletUseCases: Send + Sync {
    async fn get(&self, id: Uuid) -> Result<Wallet, ApplicationError>;
    async fn get_balance(&self, id: Uuid) -> Result<UserBalance, ApplicationError>;
    async fn list_contacts(&self, id: Uuid) -> Result<Vec<Contact>, ApplicationError>;
}
