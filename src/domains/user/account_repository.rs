use async_trait::async_trait;

use crate::application::errors::DatabaseError;

use super::{Account, AuthProvider, Permission};

#[cfg_attr(test, mockall::automock)]
#[async_trait]
pub trait AccountRepository: Send + Sync {
    async fn find_by_identity(&self, provider: AuthProvider, subject: &str) -> Result<Option<Account>, DatabaseError>;
    async fn upsert(
        &self,
        provider: AuthProvider,
        subject: &str,
        initial_permissions: &[Permission],
    ) -> Result<Account, DatabaseError>;
}
