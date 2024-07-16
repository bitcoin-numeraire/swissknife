use async_trait::async_trait;
use sea_orm::DatabaseTransaction;
use uuid::Uuid;

use crate::application::{entities::Currency, errors::DatabaseError};

use super::{Contact, Balance, Wallet};

#[async_trait]
pub trait WalletRepository: Send + Sync {
    async fn find(&self, id: Uuid) -> Result<Option<Wallet>, DatabaseError>;
    async fn find_by_user_id(&self, user_id: Uuid) -> Result<Option<Wallet>, DatabaseError>;
    async fn insert(&self, user_id: Uuid, currency: Currency) -> Result<Wallet, DatabaseError>;
    async fn get_balance(
        &self,
        txn: Option<&DatabaseTransaction>,
        id: Uuid,
    ) -> Result<Balance, DatabaseError>;
    async fn find_contacts(&self, wallet_id: Uuid) -> Result<Vec<Contact>, DatabaseError>;
}
