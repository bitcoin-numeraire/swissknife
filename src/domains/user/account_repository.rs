use async_trait::async_trait;
use uuid::Uuid;

use crate::application::errors::DatabaseError;

use super::{Account, AccountFilter, AccountPreferences, AuthProvider, Permission};

#[cfg_attr(test, mockall::automock)]
#[async_trait]
pub trait AccountRepository: Send + Sync {
    async fn find(&self, id: Uuid) -> Result<Option<Account>, DatabaseError>;
    async fn find_by_identity(&self, provider: AuthProvider, subject: &str) -> Result<Option<Account>, DatabaseError>;
    async fn find_many(&self, filter: AccountFilter) -> Result<Vec<Account>, DatabaseError>;
    async fn insert(
        &self,
        display_name: Option<String>,
        initial_permissions: &[Permission],
    ) -> Result<Account, DatabaseError>;
    async fn upsert(
        &self,
        provider: AuthProvider,
        subject: &str,
        display_name: Option<String>,
        initial_permissions: &[Permission],
    ) -> Result<Account, DatabaseError>;
    async fn update(&self, account: Account) -> Result<Account, DatabaseError>;
    async fn update_preferences(
        &self,
        id: Uuid,
        dashboard_settings: serde_json::Value,
    ) -> Result<Option<AccountPreferences>, DatabaseError>;
    async fn delete_many(&self, filter: AccountFilter) -> Result<u64, DatabaseError>;
}
