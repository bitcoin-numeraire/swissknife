use async_trait::async_trait;

use crate::{application::errors::AuthenticationError, domains::user::AuthClaims};

#[async_trait]
pub trait Authenticator: Send + Sync {
    fn generate(&self, password: &str) -> Result<String, AuthenticationError>;
    async fn decode(&self, token: &str) -> Result<AuthClaims, AuthenticationError>;
}
