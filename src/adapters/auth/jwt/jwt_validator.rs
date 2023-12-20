use std::sync::Arc;

use crate::application::errors::AuthenticationError;
use crate::{adapters::auth::Authenticator, domains::users::entities::AuthUser};
use async_trait::async_trait;
use humantime::parse_duration;
use jsonwebtoken::{
    decode, decode_header,
    jwk::{AlgorithmParameters, JwkSet},
    Algorithm, DecodingKey, Validation,
};
use serde::{Deserialize, Serialize};
use tokio::sync::RwLock;
use tokio::time::sleep;
use tracing::{error, trace};

#[derive(Clone, Debug, Deserialize)]
pub struct JWTConfig {
    domain: String,
    jwks_refresh_interval: String,
    audience: String,
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

#[derive(Clone, Debug)]
pub struct JWTValidator {
    jwks: Arc<RwLock<JwkSet>>,
    validation: Validation,
}

impl JWTValidator {
    pub async fn new(config: JWTConfig) -> Result<Self, AuthenticationError> {
        let refresh_interval = parse_duration(&config.jwks_refresh_interval)
            .map_err(|e| AuthenticationError::JWKS(e.to_string()))?;

        let jwks_uri = format!("https://{}/.well-known/jwks.json", config.domain);

        let initial_jwks = Self::fetch_jwks(&jwks_uri)
            .await
            .map_err(|e| AuthenticationError::JWKS(e.to_string()))?;

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
                    Err(e) => {
                        error!(error = ?e, jwks_uri, "Error refreshing jwks")
                    }
                }
                sleep(refresh_interval).await;
            }
        });

        let mut validation = Validation::new(Algorithm::RS256);
        validation.set_audience(&[config.audience.as_str()]);
        validation.set_issuer(&[format!("https://{}/", config.domain)]);
        validation.validate_nbf = true;
        validation.leeway = 20;

        Ok(Self { jwks, validation })
    }

    async fn fetch_jwks(jwks_uri: &str) -> Result<JwkSet, reqwest::Error> {
        Ok(reqwest::get(jwks_uri).await?.json().await?)
    }
}

#[async_trait]
impl Authenticator for JWTValidator {
    async fn validate(&self, token: &str) -> Result<AuthUser, AuthenticationError> {
        // Access the JWKs and clone the data
        let jwks = self.jwks.read().await.clone();

        let header = decode_header(token).map_err(|e| AuthenticationError::JWT(e.to_string()))?;
        let kid = match header.kid {
            Some(k) => k,
            None => {
                return Err(AuthenticationError::JWT(
                    "Token doesn't have a `kid` header field".to_string(),
                ))
            }
        };

        if let Some(j) = jwks.find(&kid) {
            match &j.algorithm {
                AlgorithmParameters::RSA(rsa) => {
                    let decoding_key = DecodingKey::from_rsa_components(&rsa.n, &rsa.e)
                        .map_err(|e| AuthenticationError::JWT(e.to_string()))?;

                    let decoded_token = decode::<Claims>(token, &decoding_key, &self.validation)
                        .map_err(|e| AuthenticationError::JWT(e.to_string()))?;

                    trace!(claims = ?decoded_token.claims, "Decoded token {:?}", decoded_token);

                    Ok(AuthUser {
                        sub: decoded_token.claims.sub,
                    })
                }
                _ => unreachable!("Only RSA algorithm is supported as JWK. should be unreachable"),
            }
        } else {
            return Err(AuthenticationError::JWT(
                "No matching JWK found for the given kid".to_string(),
            ));
        }
    }
}
