use thiserror::Error;

#[derive(Debug, Error)]
pub enum AuthenticationError {
    #[error("Failed to fetch JWKS: {0}")]
    Jwks(String),

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

    #[error("Missing Authorization header")]
    MissingAuthorizationHeader,

    #[error("Failed to generate hash from given password: {0}")]
    Hash(String),

    #[error("Failed to encode JWT token: {0}")]
    EncodeJWT(String),

    #[error("Invalid credentials")]
    InvalidCredentials,

    #[error("Unsupported operation")]
    UnsupportedOperation,
}
