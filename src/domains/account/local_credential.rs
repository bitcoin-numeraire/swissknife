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
    async fn bootstrap(&self, password_hash: String, permissions: Vec<Permission>) -> Result<Uuid, ApplicationError>;
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
}

/// OS/database operators can recover an existing login without reopening public setup.
/// Call only from the operator command, never from an unauthenticated HTTP route.
pub async fn recover_local_login(
    repository: &dyn LocalCredentialRepository,
    account_id: Uuid,
) -> Result<swissknife_types::LocalLoginReset, ApplicationError> {
    use crate::application::errors::DataError;
    use base64::{engine::general_purpose::URL_SAFE_NO_PAD, Engine};
    use serde_bolt::bitcoin::hashes::{sha256, Hash};

    let mut credential = repository
        .find(account_id)
        .await?
        .ok_or_else(|| DataError::NotFound("Existing local login not found".into()))?;
    let expected = credential.revision;
    let bytes: [u8; 32] = rand::random();
    let code = URL_SAFE_NO_PAD.encode(bytes);
    let expires_at = Utc::now() + chrono::Duration::minutes(30);
    credential.enabled = true;
    credential.password_hash = None;
    credential.reset_hash = Some(sha256::Hash::hash(code.as_bytes()).to_string());
    credential.reset_expires_at = Some(expires_at);
    credential.revision = Uuid::new_v4();
    repository.replace(credential, expected).await?;
    Ok(swissknife_types::LocalLoginReset { code, expires_at })
}
