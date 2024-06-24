use std::time::Duration;

use crate::application::errors::AuthenticationError;
use crate::{domains::users::entities::AuthUser, infra::auth::Authenticator};
use async_trait::async_trait;

use serde::{Deserialize, Serialize};

use crate::infra::config::config_rs::deserialize_duration;

#[derive(Clone, Debug, Deserialize)]
pub struct JwtConfig {
    username: String,
    password: String,
    #[serde(deserialize_with = "deserialize_duration")]
    token_expiration: Duration,
}

#[derive(Debug, Serialize, Deserialize)]
struct Claims {
    aud: Vec<String>, // Optional. Audience
    exp: usize, // Required (validate_exp defaults to true in validation). Expiration time (as UTC timestamp)
    iat: usize, // Optional. Issued at (as UTC timestamp)
    iss: String, // Optional. Issuer
    sub: String, // Optional. Subject (whom token refers to)
    permissions: Option<Vec<String>>,
}

#[derive(Clone, Debug)]
pub struct JwtAuthenticator {
    token_expiration: Duration,
}

impl JwtAuthenticator {
    pub async fn new(config: JwtConfig) -> Result<Self, AuthenticationError> {
        Ok(Self {
            token_expiration: config.token_expiration,
        })
    }
}

#[async_trait]
impl Authenticator for JwtAuthenticator {
    async fn authenticate(&self, token: &str) -> Result<AuthUser, AuthenticationError> {
        todo!("Implement JWT authentication")
    }
}
