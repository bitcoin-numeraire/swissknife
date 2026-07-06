use async_trait::async_trait;
use uuid::Uuid;

use crate::application::errors::DatabaseError;

use super::{Account, Permission};

#[cfg_attr(test, mockall::automock)]
#[async_trait]
pub trait AccountRepository: Send + Sync {
    async fn find_by_identity(&self, provider: &str, subject: &str) -> Result<Option<Account>, DatabaseError>;
    async fn upsert_for_identity(&self, provider: &str, subject: &str) -> Result<Account, DatabaseError>;
    async fn upsert_permissions(&self, account_id: Uuid, permissions: &[Permission]) -> Result<(), DatabaseError>;
    async fn find_permissions(&self, account_id: Uuid) -> Result<Vec<Permission>, DatabaseError>;
}
