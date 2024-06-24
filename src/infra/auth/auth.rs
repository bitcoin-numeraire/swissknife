use async_trait::async_trait;

use crate::{application::errors::AuthenticationError, domains::users::entities::AuthUser};

#[async_trait]
pub trait Authenticator: Send + Sync {
    async fn authenticate(&self, token: &str) -> Result<AuthUser, AuthenticationError>;
}
