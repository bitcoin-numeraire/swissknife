use async_trait::async_trait;

use crate::{
    application::errors::AuthenticationError,
    domains::user::{AuthClaims, Permission},
};

#[async_trait]
pub trait JWTAuthenticator: Send + Sync {
    fn encode(
        &self,
        username: String,
        permissions: Vec<Permission>,
    ) -> Result<String, AuthenticationError>;
    async fn decode(&self, token: &str) -> Result<AuthClaims, AuthenticationError>;
}
