use async_trait::async_trait;

use crate::{application::errors::ApplicationError, domains::users::entities::AuthUser};

#[async_trait]
pub trait UserUseCases: Send + Sync {
    async fn login(&self, password: String) -> Result<String, ApplicationError>;
    async fn authenticate_jwt(&self, token: String) -> Result<AuthUser, ApplicationError>;
}
