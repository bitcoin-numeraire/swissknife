use async_trait::async_trait;
use serde::Deserialize;

use crate::application::errors::AuthenticationError;

use super::jwt::JWTConfig;

#[derive(Clone, Debug, Deserialize)]
pub struct AuthConfig {
    pub enabled: bool,
    pub jwt: JWTConfig,
}

#[async_trait]
pub trait Authenticator {
    async fn validate(&self, token: &str) -> Result<(), AuthenticationError>;
}
