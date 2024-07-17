use async_trait::async_trait;
use sea_orm::DatabaseTransaction;
use uuid::Uuid;

use crate::application::errors::DatabaseError;

use super::{Balance, Contact, Wallet};

#[async_trait]
pub trait WalletRepository: Send + Sync {
    async fn find(&self, id: Uuid) -> Result<Option<Wallet>, DatabaseError>;
    async fn find_by_user_id(&self, user_id: &str) -> Result<Option<Wallet>, DatabaseError>;
    async fn find_many(&self) -> Result<Vec<Wallet>, DatabaseError>;
    async fn insert(&self, user_id: &str) -> Result<Wallet, DatabaseError>;
    async fn get_balance(
        &self,
        txn: Option<&DatabaseTransaction>,
        id: Uuid,
    ) -> Result<Balance, DatabaseError>;
    async fn find_contacts(&self, id: Uuid) -> Result<Vec<Contact>, DatabaseError>;
}
