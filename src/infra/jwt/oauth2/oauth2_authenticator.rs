use std::sync::Arc;
use std::time::Duration;

use crate::domains::account::AuthClaims;
use crate::infra::jwt::JWTAuthenticator;
use crate::{application::errors::AuthenticationError, domains::account::Account};
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
    /// Issuer base URL of the OpenID provider. It may carry an explicit scheme
    /// (e.g. an internal IdP reachable only over `http`); when the scheme is
    /// omitted, `https` is assumed — matching how hosted providers such as Auth0
    /// are configured (`domain = "auth.example.com"`). The OpenID Connect
    /// discovery document is fetched from this base to learn the JWKS URI and
    /// the canonical issuer used for validation.
    domain: String,
    #[serde(deserialize_with = "deserialize_duration")]
    jwks_refresh_interval: Duration,
    audience: String,
    #[serde(deserialize_with = "deserialize_duration")]
    leeway: Duration,
}

impl OAuth2Config {
    /// The issuer base URL with a scheme guaranteed and any trailing slash
    /// stripped. A bare host defaults to `https`.
    fn issuer_base(&self) -> String {
        let trimmed = self.domain.trim_end_matches('/');
        if trimmed.contains("://") {
            trimmed.to_string()
        } else {
            format!("https://{trimmed}")
        }
    }
}

/// The subset of the OpenID Provider Metadata (OpenID Connect Discovery /
/// RFC 8414) that we rely on. Unknown fields are ignored.
#[derive(Clone, Debug, Deserialize)]
struct OpenIdProviderMetadata {
    issuer: String,
    jwks_uri: String,
}

/// Per OpenID Connect Discovery, the `issuer` advertised in the metadata must be
/// identical to the issuer used to fetch it. Enforcing this stops a rogue or
/// misconfigured `.well-known` from substituting a different issuer (and its
/// JWKS) that we would then trust. Hosted providers (e.g. Auth0) advertise the
/// issuer with a trailing slash while we configure the bare origin, so the
/// comparison normalizes trailing slashes.
fn verify_discovered_issuer(configured_base: &str, discovered: &str) -> Result<(), AuthenticationError> {
    if discovered.trim_end_matches('/') == configured_base.trim_end_matches('/') {
        Ok(())
    } else {
        Err(AuthenticationError::Discovery(format!(
            "issuer mismatch: discovery at {configured_base} advertised issuer {discovered:?}"
        )))
    }
}

#[derive(Clone, Debug)]
pub struct OAuth2Authenticator {
    jwks: Arc<RwLock<JwkSet>>,
    validation: Validation,
}

impl OAuth2Authenticator {
    pub async fn new(config: OAuth2Config) -> Result<Self, AuthenticationError> {
        let issuer_base = config.issuer_base();
        let discovery_url = format!("{issuer_base}/.well-known/openid-configuration");

        // Resolve the JWKS URI and canonical issuer via OpenID discovery, then
        // verify the advertised issuer matches the one we discovered from before
        // trusting it (and the JWKS it points at) for token validation.
        let metadata = Self::fetch_discovery(&discovery_url)
            .await
            .map_err(|e| AuthenticationError::Discovery(e.to_string()))?;
        verify_discovered_issuer(&issuer_base, &metadata.issuer)?;

        let jwks_uri = metadata.jwks_uri;
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
        validation.set_issuer(&[metadata.issuer.as_str()]);
        validation.leeway = config.leeway.as_secs();

        Ok(Self { jwks, validation })
    }

    async fn fetch_discovery(discovery_url: &str) -> Result<OpenIdProviderMetadata, reqwest::Error> {
        let metadata = reqwest::get(discovery_url).await?.error_for_status()?.json().await?;
        Ok(metadata)
    }

    async fn fetch_jwks(jwks_uri: &str) -> Result<JwkSet, reqwest::Error> {
        let jwks = reqwest::get(jwks_uri).await?.error_for_status()?.json().await?;
        Ok(jwks)
    }
}

#[async_trait]
impl JWTAuthenticator for OAuth2Authenticator {
    fn encode(&self, _: Account) -> Result<String, AuthenticationError> {
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

#[cfg(test)]
mod tests {
    use super::*;

    fn config(domain: &str) -> OAuth2Config {
        OAuth2Config {
            domain: domain.to_string(),
            jwks_refresh_interval: Duration::from_secs(3600),
            audience: "https://api.example.com".to_string(),
            leeway: Duration::from_secs(60),
        }
    }

    mod issuer_base {
        use super::*;

        mod when_no_scheme_is_given {
            use super::*;

            #[test]
            fn defaults_to_https() {
                assert_eq!(config("auth.example.com").issuer_base(), "https://auth.example.com");
            }
        }

        mod when_a_scheme_is_given {
            use super::*;

            #[test]
            fn preserves_the_scheme_and_path() {
                assert_eq!(
                    config("http://127.0.0.1:8090/default").issuer_base(),
                    "http://127.0.0.1:8090/default"
                );
            }
        }

        mod with_a_trailing_slash {
            use super::*;

            #[test]
            fn is_stripped() {
                assert_eq!(
                    config("https://auth.example.com/").issuer_base(),
                    "https://auth.example.com"
                );
                assert_eq!(config("auth.example.com/").issuer_base(), "https://auth.example.com");
            }
        }
    }

    mod discovered_issuer {
        use super::*;

        #[test]
        fn accepts_an_exact_match() {
            let issuer = "http://127.0.0.1:8090/default";
            assert!(verify_discovered_issuer(issuer, issuer).is_ok());
        }

        #[test]
        fn accepts_a_trailing_slash_difference() {
            // Auth0 advertises its issuer with a trailing slash; we configure the bare origin.
            assert!(verify_discovered_issuer("https://auth.example.com", "https://auth.example.com/").is_ok());
        }

        #[test]
        fn rejects_a_different_issuer() {
            let err = verify_discovered_issuer("https://auth.example.com", "https://evil.example.com/").unwrap_err();
            assert!(matches!(err, AuthenticationError::Discovery(_)));
        }
    }

    mod openid_provider_metadata {
        use super::*;

        #[test]
        fn deserializes_and_ignores_unknown_fields() {
            let json = r#"{
                "issuer": "https://auth.example.com/",
                "jwks_uri": "https://auth.example.com/.well-known/jwks.json",
                "authorization_endpoint": "https://auth.example.com/authorize",
                "response_types_supported": ["code"]
            }"#;

            let metadata: OpenIdProviderMetadata = serde_json::from_str(json).unwrap();

            assert_eq!(metadata.issuer, "https://auth.example.com/");
            assert_eq!(metadata.jwks_uri, "https://auth.example.com/.well-known/jwks.json");
        }
    }
}
