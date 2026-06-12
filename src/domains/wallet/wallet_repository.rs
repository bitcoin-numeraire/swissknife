use async_trait::async_trait;
use uuid::Uuid;

use crate::application::{composition::Currency, errors::DatabaseError};

use super::{Balance, Contact, Wallet, WalletFilter, WalletOverview};

#[cfg_attr(test, mockall::automock)]
#[async_trait]
pub trait WalletRepository: Send + Sync {
    async fn find(&self, id: Uuid, currency: &Currency) -> Result<Option<Wallet>, DatabaseError>;
    async fn find_by_user_id(&self, user_id: &str) -> Result<Option<Wallet>, DatabaseError>;
    async fn find_many(&self, filter: WalletFilter) -> Result<Vec<Wallet>, DatabaseError>;
    async fn find_many_overview(&self, currency: &Currency) -> Result<Vec<WalletOverview>, DatabaseError>;
    async fn insert(&self, user_id: &str) -> Result<Wallet, DatabaseError>;
    async fn get_balance(&self, id: Uuid, currency: &Currency) -> Result<Balance, DatabaseError>;
    async fn find_contacts(&self, id: Uuid) -> Result<Vec<Contact>, DatabaseError>;
    async fn delete_many(&self, filter: WalletFilter) -> Result<u64, DatabaseError>;
}
