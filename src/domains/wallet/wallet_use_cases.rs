use async_trait::async_trait;
use uuid::Uuid;

use crate::application::errors::ApplicationError;

use super::{Balance, Contact, Wallet, WalletFilter, WalletOverview};

#[async_trait]
pub trait WalletUseCases: Send + Sync {
    async fn register(&self, user_id: String) -> Result<Wallet, ApplicationError>;
    async fn get(&self, id: Uuid) -> Result<Wallet, ApplicationError>;
    async fn list(&self, filter: WalletFilter) -> Result<Vec<Wallet>, ApplicationError>;
    async fn list_overviews(&self) -> Result<Vec<WalletOverview>, ApplicationError>;
    async fn get_balance(&self, id: Uuid) -> Result<Balance, ApplicationError>;
    async fn list_contacts(&self, id: Uuid) -> Result<Vec<Contact>, ApplicationError>;
    async fn delete(&self, id: Uuid) -> Result<(), ApplicationError>;
    async fn delete_many(&self, filter: WalletFilter) -> Result<u64, ApplicationError>;
}
