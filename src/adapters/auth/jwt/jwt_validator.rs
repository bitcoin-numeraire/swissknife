use std::sync::Arc;

use crate::adapters::auth::Authenticator;
use crate::application::errors::AuthenticationError;
use humantime::parse_duration;
use jsonwebtoken::jwk::JwkSet;
use serde::Deserialize;
use serde::Serialize;
use tokio::sync::RwLock;
use tokio::time::sleep;
use tracing::error;
use tracing::trace;

#[derive(Clone, Debug, Deserialize)]
pub struct JWTConfig {
    domain: String,
    audience: String,
    jwks_uri: String,
    algorithm: String,
    issuer: String,
    jwks_refresh_interval: String,
}

#[derive(Debug, Serialize, Deserialize)]
struct Claims {
    aud: String, // Optional. Audience
    exp: usize, // Required (validate_exp defaults to true in validation). Expiration time (as UTC timestamp)
    iat: usize, // Optional. Issued at (as UTC timestamp)
    iss: String, // Optional. Issuer
    nbf: usize, // Optional. Not Before (as UTC timestamp)
    sub: String, // Optional. Subject (whom token refers to)
}

pub struct JWTValidator {
    jwks: Arc<RwLock<JwkSet>>,
}

impl JWTValidator {
    pub async fn new(config: JWTConfig) -> Result<Self, AuthenticationError> {
        let refresh_interval = parse_duration(&config.jwks_refresh_interval)
            .map_err(|e| AuthenticationError::RefreshInterval(e.to_string()))?;

        let initial_jwks = Self::refresh_jwks(&config.jwks_uri)
            .await
            .map_err(|e| AuthenticationError::JWKS(e.to_string()))?;

        let jwks = Arc::new(RwLock::new(initial_jwks));
        let jwks_clone = Arc::clone(&jwks);

        tokio::spawn(async move {
            loop {
                match Self::refresh_jwks(&config.jwks_uri).await {
                    Ok(new_jwks) => {
                        let mut jwks_write = jwks_clone.write().await;
                        *jwks_write = new_jwks;
                        trace!(jwks_uri = %config.jwks_uri, "Refreshed JWKS");
                    }
                    Err(e) => {
                        error!(error = ?e, jwks_uri = %config.jwks_uri, "Error refreshing jwks")
                    }
                }
                sleep(refresh_interval).await;
            }
        });

        Ok(Self { jwks })
    }

    async fn refresh_jwks(jwks_uri: &str) -> Result<JwkSet, reqwest::Error> {
        let jwks: JwkSet = reqwest::get(jwks_uri).await?.json().await?;

        Ok(jwks)
    }
}

impl Authenticator for JWTValidator {}
