use async_trait::async_trait;
use sea_orm::DatabaseTransaction;
use uuid::Uuid;

use crate::{
    application::errors::DatabaseError,
    domains::lightning::entities::{LightningAddress, LightningAddressFilter, UserBalance},
};

#[async_trait]
pub trait WalletRepository {
    async fn get_balance(
        &self,
        txn: Option<&DatabaseTransaction>,
        user: &str,
    ) -> Result<UserBalance, DatabaseError>;
}

#[async_trait]
pub trait LightningAddressRepository {
    async fn find_address(&self, id: Uuid) -> Result<Option<LightningAddress>, DatabaseError>;
    async fn find_address_by_username(
        &self,
        username: &str,
    ) -> Result<Option<LightningAddress>, DatabaseError>;
    async fn find_address_by_user_id(
        &self,
        user: &str,
    ) -> Result<Option<LightningAddress>, DatabaseError>;
    async fn find_addresses(
        &self,
        filter: LightningAddressFilter,
    ) -> Result<Vec<LightningAddress>, DatabaseError>;
    async fn insert_address(
        &self,
        user: &str,
        username: &str,
    ) -> Result<LightningAddress, DatabaseError>;
    async fn delete_addresses(&self, filter: LightningAddressFilter) -> Result<u64, DatabaseError>;
}

#[async_trait]
pub trait TransactionManager {
    async fn begin(&self) -> Result<DatabaseTransaction, DatabaseError>;
}

pub trait LightningRepository:
    LightningAddressRepository + WalletRepository + TransactionManager + Sync + Send
{
}

// Ensure that any type that implements the individual traits also implements the new trait.
impl<T> LightningRepository for T where
    T: LightningAddressRepository + WalletRepository + TransactionManager + Sync + Send
{
}
