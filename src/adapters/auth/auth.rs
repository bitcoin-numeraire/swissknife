use std::sync::Arc;

use async_trait::async_trait;
use serde::Deserialize;

use crate::{application::errors::AuthenticationError, domains::users::entities::AuthUser};

use super::jwt::JWTConfig;

#[derive(Clone, Debug, Deserialize)]
pub struct AuthConfig {
    pub enabled: bool,
    pub jwt: JWTConfig,
}

#[async_trait]
pub trait Authenticator {
    async fn validate(&self, token: &str) -> Result<AuthUser, AuthenticationError>;
}

pub type DynAuthenticator = Arc<dyn Authenticator + Send + Sync>;
