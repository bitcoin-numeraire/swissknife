use std::sync::Arc;
use std::time::Duration;

use crate::application::errors::AuthenticationError;
use crate::domains::users::entities::Permission;
use crate::{domains::users::entities::AuthUser, infra::auth::Authenticator};
use async_trait::async_trait;
use jsonwebtoken::{
    decode, decode_header,
    jwk::{AlgorithmParameters, JwkSet},
    Algorithm, DecodingKey, Validation,
};
use serde::{Deserialize, Serialize};
use tokio::sync::RwLock;
use tokio::time::sleep;
use tracing::{error, trace};

use crate::infra::config::config_rs::deserialize_duration;

#[derive(Clone, Debug, Deserialize)]
pub struct OAuth2Config {
    domain: String,
    #[serde(deserialize_with = "deserialize_duration")]
    jwks_refresh_interval: Duration,
    audience: String,
    #[serde(deserialize_with = "deserialize_duration")]
    leeway: Duration,
}

#[derive(Debug, Serialize, Deserialize)]
struct Claims {
    aud: Vec<String>, // Optional. Audience
    exp: usize, // Required (validate_exp defaults to true in validation). Expiration time (as UTC timestamp)
    iat: usize, // Optional. Issued at (as UTC timestamp)
    iss: String, // Optional. Issuer
    sub: String, // Optional. Subject (whom token refers to)
    permissions: Vec<Permission>,
}

#[derive(Clone, Debug)]
pub struct OAuth2Authenticator {
    jwks: Arc<RwLock<JwkSet>>,
    validation: Validation,
}

impl OAuth2Authenticator {
    pub async fn new(config: OAuth2Config) -> Result<Self, AuthenticationError> {
        let jwks_uri = format!("https://{}/.well-known/jwks.json", config.domain);

        let initial_jwks = Self::fetch_jwks(&jwks_uri)
            .await
            .map_err(|e| AuthenticationError::Jwks(e.to_string()))?;

        let jwks = Arc::new(RwLock::new(initial_jwks));
        let jwks_clone = Arc::clone(&jwks);

        tokio::spawn(async move {
            loop {
                match Self::fetch_jwks(&jwks_uri).await {
                    Ok(new_jwks) => {
                        let mut jwks_write = jwks_clone.write().await;
                        *jwks_write = new_jwks;
                        trace!(jwks_uri, "Refreshed JWKS");
                    }
                    Err(err) => {
                        error!(%err, jwks_uri, "Error refreshing jwks")
                    }
                }
                sleep(config.jwks_refresh_interval).await;
            }
        });

        let mut validation = Validation::new(Algorithm::RS256);
        validation.set_audience(&[config.audience.as_str()]);
        validation.set_issuer(&[format!("https://{}/", config.domain)]);
        validation.leeway = config.leeway.as_secs();

        Ok(Self { jwks, validation })
    }

    async fn fetch_jwks(jwks_uri: &str) -> Result<JwkSet, reqwest::Error> {
        let jwks = reqwest::get(jwks_uri).await?.json().await?;
        Ok(jwks)
    }
}

#[async_trait]
impl Authenticator for OAuth2Authenticator {
    fn generate_jwt_token(&self, _: &str) -> Result<String, AuthenticationError> {
        Err(AuthenticationError::UnsupportedOperation)
    }

    async fn authenticate(&self, token: &str) -> Result<AuthUser, AuthenticationError> {
        // Access the JWKs and clone the data
        let jwks = self.jwks.read().await.clone();

        let header = decode_header(token)
            .map_err(|e| AuthenticationError::DecodeJWTHeader(e.to_string()))?;
        let kid = match header.kid {
            Some(k) => k,
            None => {
                return Err(AuthenticationError::MissingJWTKid);
            }
        };

        if let Some(j) = jwks.find(&kid) {
            match &j.algorithm {
                AlgorithmParameters::RSA(rsa) => {
                    let decoding_key = DecodingKey::from_rsa_components(&rsa.n, &rsa.e)
                        .map_err(|e| AuthenticationError::DecodeJWTKey(e.to_string()))?;

                    let decoded_token = decode::<Claims>(token, &decoding_key, &self.validation)
                        .map_err(|e| AuthenticationError::DecodeJWT(e.to_string()))?;

                    Ok(AuthUser {
                        sub: decoded_token.claims.sub,
                        permissions: decoded_token.claims.permissions,
                    })
                }
                _ => unreachable!("Only RSA algorithm is supported as JWK. should be unreachable"),
            }
        } else {
            Err(AuthenticationError::MissingJWK)
        }
    }
}
