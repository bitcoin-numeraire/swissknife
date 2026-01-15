use std::sync::Arc;
use std::time::Duration;

use crate::domains::user::AuthClaims;
use crate::infra::jwt::JWTAuthenticator;
use crate::{application::errors::AuthenticationError, domains::user::Permission};
use async_trait::async_trait;
use jsonwebtoken::{
    decode, decode_header,
    jwk::{AlgorithmParameters, JwkSet},
    Algorithm, DecodingKey, Validation,
};
use serde::Deserialize;
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
impl JWTAuthenticator for OAuth2Authenticator {
    fn encode(&self, _: String, _: Vec<Permission>) -> Result<String, AuthenticationError> {
        Err(AuthenticationError::UnsupportedOperation)
    }

    async fn decode(&self, token: &str) -> Result<AuthClaims, AuthenticationError> {
        // Access the JWKs and clone the data
        let jwks = self.jwks.read().await.clone();

        let header = decode_header(token).map_err(|e| AuthenticationError::DecodeJWTHeader(e.to_string()))?;
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

                    let decoded_token = decode::<AuthClaims>(token, &decoding_key, &self.validation)
                        .map_err(|e| AuthenticationError::DecodeJWT(e.to_string()))?;

                    Ok(decoded_token.claims)
                }
                _ => unreachable!("Only RSA algorithm is supported as JWK. should be unreachable"),
            }
        } else {
            Err(AuthenticationError::MissingJWK)
        }
    }
}
