use async_trait::async_trait;

use crate::{
    application::errors::AuthenticationError,
    domains::user::{Account, AuthClaims},
};

#[cfg_attr(test, mockall::automock)]
#[async_trait]
pub trait JWTAuthenticator: Send + Sync {
    fn encode(&self, account: Account) -> Result<String, AuthenticationError>;
    async fn decode(&self, token: &str) -> Result<AuthClaims, AuthenticationError>;
}
