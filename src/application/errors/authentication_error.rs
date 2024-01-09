use thiserror::Error;

#[derive(Debug, Error)]
pub enum AuthenticationError {
    #[error("Failed to parse config: {0}")]
    ParseConfig(String),

    #[error("Failed to fetch JWKS: {0}")]
    JWKS(String),

    #[error("Invalid JWT token: {0}")]
    JWT(String),

    #[error("Missing Bearer token for authentication: {0}")]
    MissingBearerToken(String),
}
