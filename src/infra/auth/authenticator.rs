use async_trait::async_trait;

use crate::{application::errors::AuthenticationError, domains::user::AuthUser};

#[async_trait]
pub trait Authenticator: Send + Sync {
    fn generate_jwt_token(&self, password: &str) -> Result<String, AuthenticationError>;
    async fn authenticate(&self, token: &str) -> Result<AuthUser, AuthenticationError>;
}
