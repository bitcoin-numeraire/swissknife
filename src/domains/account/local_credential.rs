use async_trait::async_trait;
use chrono::{DateTime, Utc};
use uuid::Uuid;

use super::Permission;
use crate::application::errors::ApplicationError;

pub const LOCAL_AUTH_INITIALIZED_KEY: &str = "local_auth_initialized";

/// Authentication-only data. Never serialize or log this structure.
pub struct LocalCredential {
    pub account_id: Uuid,
    pub identity_id: Uuid,
    pub subject: String,
    pub password_hash: Option<String>,
    pub enabled: bool,
    pub revision: Uuid,
    pub reset_hash: Option<String>,
    pub reset_expires_at: Option<DateTime<Utc>>,
}

#[cfg_attr(test, mockall::automock)]
#[async_trait]
pub trait LocalCredentialRepository: Send + Sync {
    async fn find(&self, account_id: Uuid) -> Result<Option<LocalCredential>, ApplicationError>;
    async fn find_by_subject(&self, subject: &str) -> Result<Option<LocalCredential>, ApplicationError>;
    async fn find_by_reset_hash(&self, hash: &str) -> Result<Option<LocalCredential>, ApplicationError>;
    /// Atomically claims the permanent setup marker and creates the owner aggregate.
    async fn bootstrap(
        &self,
        username: String,
        display_name: Option<String>,
        password_hash: String,
        permissions: Vec<Permission>,
    ) -> Result<Uuid, ApplicationError>;
    /// Atomically links one local identity and its credential to an existing account.
    async fn create(
        &self,
        account_id: Uuid,
        subject: String,
        reset_hash: String,
        reset_expires_at: DateTime<Utc>,
    ) -> Result<(), ApplicationError>;
    /// Compare-and-swap prevents an in-flight password operation undoing a reset or disable.
    async fn replace(&self, credential: LocalCredential, expected_revision: Uuid) -> Result<(), ApplicationError>;
    /// Consume a reset grant and optionally update the display name in the same transaction.
    /// None preserves the name; Some(None) clears it.
    async fn reset_password(
        &self,
        credential: LocalCredential,
        expected_revision: Uuid,
        display_name: Option<Option<String>>,
    ) -> Result<(), ApplicationError>;
}
