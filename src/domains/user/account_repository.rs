use async_trait::async_trait;

use crate::application::errors::DatabaseError;

use super::Account;

#[async_trait]
pub trait AccountRepository: Send + Sync {
    async fn find_by_sub(&self, sub: &str) -> Result<Option<Account>, DatabaseError>;
    async fn insert(&self, sub: &str) -> Result<Account, DatabaseError>;
}
