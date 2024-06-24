use async_trait::async_trait;

use crate::{application::errors::ApplicationError, domains::users::entities::AuthUser};

#[async_trait]
pub trait UserUseCases: Send + Sync {
    fn login(&self, password: String) -> Result<String, ApplicationError>;
    async fn authenticate_jwt(&self, token: &str) -> Result<AuthUser, ApplicationError>;
}
