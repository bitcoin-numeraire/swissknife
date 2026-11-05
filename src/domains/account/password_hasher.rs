use async_trait::async_trait;

use crate::application::errors::ApplicationError;

/// Password hashing and verification adapter. Password policy belongs to the use case.
#[cfg_attr(test, mockall::automock)]
#[async_trait]
pub trait PasswordHasher: Send + Sync {
    async fn hash(&self, password: String) -> Result<String, ApplicationError>;
    /// Missing credentials must still perform verification work and return false.
    async fn verify(&self, password: String, stored: Option<String>) -> Result<bool, ApplicationError>;
}
