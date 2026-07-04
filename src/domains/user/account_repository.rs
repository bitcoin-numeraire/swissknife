use async_trait::async_trait;
use uuid::Uuid;

use crate::application::errors::DatabaseError;

use super::{AccountIdentity, Permission};

#[cfg_attr(test, mockall::automock)]
#[async_trait]
pub trait AccountRepository: Send + Sync {
    async fn ensure_for_identity(&self, provider: &str, subject: &str) -> Result<AccountIdentity, DatabaseError>;
    async fn grant_permissions(&self, account_id: Uuid, permissions: &[Permission]) -> Result<(), DatabaseError>;
}
