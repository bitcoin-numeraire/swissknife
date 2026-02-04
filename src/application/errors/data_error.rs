use thiserror::Error;
use utoipa::ToSchema;

#[derive(Debug, Error, ToSchema)]
pub enum DataError {
    #[error("Conflict: {0}")]
    Conflict(String),

    #[error("Not Found: {0}")]
    NotFound(String),

    #[error("Malformed: {0}")]
    Malformed(String),

    #[error("Validation failed: {0}")]
    Validation(String),

    // TODO: Might make sense to move to a different error type such as AccountError
    #[error("Insufficient funds. Required: {0} mSats.")]
    InsufficientFunds(f64),

    #[error("Data inconsistency: {0}. Please contact support.")]
    Inconsistency(String),
}
