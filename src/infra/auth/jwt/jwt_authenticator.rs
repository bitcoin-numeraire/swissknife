use std::time::Duration;

use crate::domains::user::AuthClaims;
use crate::infra::auth::Authenticator;
use crate::{application::errors::AuthenticationError, domains::user::Permission};
use async_trait::async_trait;

use bcrypt::{hash, verify, DEFAULT_COST};
use jsonwebtoken::{decode, encode, DecodingKey, EncodingKey, Header, Validation};
use serde::{Deserialize, Serialize};

use crate::infra::config::config_rs::deserialize_duration;

#[derive(Clone, Debug, Deserialize)]
pub struct JwtConfig {
    username: String,
    password: String,
    #[serde(deserialize_with = "deserialize_duration")]
    token_expiry: Duration,
}

#[derive(Clone)]
pub struct JwtAuthenticator {
    username: String,
    password_hash: String,
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

impl JwtAuthenticator {
    pub async fn new(config: JwtConfig) -> Result<Self, AuthenticationError> {
        let password_hash = hash(&config.password, DEFAULT_COST)
            .map_err(|e| AuthenticationError::Hash(e.to_string()))?;

        Ok(Self {
            username: config.username,
            password_hash,
            token_expiry: config.token_expiry,
            encoding_key: EncodingKey::from_secret(config.password.as_ref()),
            decoding_key: DecodingKey::from_secret(config.password.as_ref()),
        })
    }
}

#[async_trait]
impl Authenticator for JwtAuthenticator {
    fn generate(&self, password: &str) -> Result<String, AuthenticationError> {
        if !verify(password, &self.password_hash)
            .map_err(|e| AuthenticationError::Hash(e.to_string()))?
        {
            return Err(AuthenticationError::InvalidCredentials);
        }

        let now = chrono::Utc::now().timestamp();
        let expiration = now + self.token_expiry.as_secs() as i64;

        let claims = Claims {
            sub: self.username.clone(),
            exp: expiration as usize,
            iat: now as usize,
            permissions: Permission::all_permissions(),
        };

        let token = encode(&Header::default(), &claims, &self.encoding_key)
            .map_err(|e| AuthenticationError::EncodeJWT(e.to_string()))?;

        Ok(token)
    }

    async fn decode(&self, token: &str) -> Result<AuthClaims, AuthenticationError> {
        let token_data = decode::<Claims>(token, &self.decoding_key, &Validation::default())
            .map_err(|e| AuthenticationError::DecodeJWT(e.to_string()))?;

        Ok(AuthClaims {
            sub: token_data.claims.sub,
            permissions: token_data.claims.permissions,
        })
    }
}
