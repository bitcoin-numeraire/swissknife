use async_trait::async_trait;

use crate::application::{dtos::AuthProvider, errors::ApplicationError};

use super::User;

#[async_trait]
pub trait AuthUseCases: Send + Sync {
    fn sign_in(&self, password: String) -> Result<String, ApplicationError>;
    async fn authenticate(&self, token: &str) -> Result<User, ApplicationError>;
    fn provider(&self) -> AuthProvider;
}
