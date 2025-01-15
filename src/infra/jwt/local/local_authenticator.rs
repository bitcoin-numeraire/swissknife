use std::time::Duration;

use crate::domains::user::AuthClaims;
use crate::infra::jwt::JWTAuthenticator;
use crate::{application::errors::AuthenticationError, domains::user::Permission};
use async_trait::async_trait;

use jsonwebtoken::{decode, encode, DecodingKey, EncodingKey, Header, Validation};
use serde::{Deserialize, Serialize};

use crate::infra::config::config_rs::deserialize_duration;

#[derive(Clone, Debug, Deserialize)]
pub struct JwtConfig {
    #[serde(deserialize_with = "deserialize_duration")]
    token_expiry: Duration,
    secret: String,
}

#[derive(Clone)]
pub struct LocalAuthenticator {
    token_expiry: Duration,
    encoding_key: EncodingKey,
    decoding_key: DecodingKey,
}

#[derive(Debug, Serialize, Deserialize)]
struct Claims {
    exp: usize, // Required (validate_exp defaults to true in validation). Expiration time (as UTC timestamp)
    iat: usize, // Optional. Issued at (as UTC timestamp)
    sub: String, // Optional. Subject (whom token refers to)
    permissions: Vec<Permission>,
}

impl LocalAuthenticator {
    pub async fn new(config: JwtConfig) -> Result<Self, AuthenticationError> {
        Ok(Self {
            token_expiry: config.token_expiry,
            encoding_key: EncodingKey::from_secret(config.secret.as_ref()),
            decoding_key: DecodingKey::from_secret(config.secret.as_ref()),
        })
    }
}

#[async_trait]
impl JWTAuthenticator for LocalAuthenticator {
    fn encode(
        &self,
        sub: String,
        permissions: Vec<Permission>,
    ) -> Result<String, AuthenticationError> {
        let now = chrono::Utc::now().timestamp();
        let expiration = now + self.token_expiry.as_secs() as i64;

        let claims = AuthClaims {
            sub,
            exp: expiration as usize,
            iat: now as usize,
            permissions,
        };

        let token = encode(&Header::default(), &claims, &self.encoding_key)
            .map_err(|e| AuthenticationError::EncodeJWT(e.to_string()))?;

        Ok(token)
    }

    async fn decode(&self, token: &str) -> Result<AuthClaims, AuthenticationError> {
        let token_data = decode::<AuthClaims>(token, &self.decoding_key, &Validation::default())
            .map_err(|e| AuthenticationError::DecodeJWT(e.to_string()))?;

        Ok(token_data.claims)
    }
}
