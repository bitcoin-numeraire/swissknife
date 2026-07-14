use std::time::Duration;

use crate::application::errors::AuthenticationError;
use crate::domains::account::{Account, AuthClaims};
use crate::infra::jwt::JWTAuthenticator;
use async_trait::async_trait;

use jsonwebtoken::{decode, encode, DecodingKey, EncodingKey, Header, Validation};
use serde::Deserialize;

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
    fn encode(&self, account: Account) -> Result<String, AuthenticationError> {
        let now = chrono::Utc::now().timestamp();
        let expiration = now + self.token_expiry.as_secs() as i64;
        let identity = account
            .identity
            .ok_or_else(|| AuthenticationError::EncodeJWT(format!("Account {} has no identity", account.id)))?;

        let claims = AuthClaims {
            sub: identity.subject,
            exp: expiration as usize,
            iat: now as usize,
            permissions: account.permissions.unwrap_or_default(),
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
