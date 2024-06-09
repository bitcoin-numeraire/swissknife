use thiserror::Error;

#[derive(Debug, Error)]
pub enum AuthenticationError {
    #[error("Failed to fetch JWKS: {0}")]
    JWKS(String),

    #[error("Failed to decode JWT header: {0}")]
    DecodeJWTHeader(String),

    #[error("Failed to decode JWT key: {0}")]
    DecodeJWTKey(String),

    #[error("Failed to decode JWT token: {0}")]
    DecodeJWT(String),

    #[error("Missing `kid` header field")]
    MissingJWTKid,

    #[error("No matching JWK found for the given kid")]
    MissingJWK,

    #[error("Missing Bearer token for authentication: {0}")]
    MissingBearerToken(String),
}
