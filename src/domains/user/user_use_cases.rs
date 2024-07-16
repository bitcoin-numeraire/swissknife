use async_trait::async_trait;

use crate::application::{dtos::AuthProvider, errors::ApplicationError};

use super::AuthUser;

#[async_trait]
pub trait UserUseCases: Send + Sync {
    fn sign_in(&self, password: String) -> Result<String, ApplicationError>;
    async fn authenticate(&self, token: &str) -> Result<AuthUser, ApplicationError>;
    fn provider(&self) -> AuthProvider;
}
