use thiserror::Error;

#[derive(Debug, Error)]
pub enum DataError {
    #[error("Conflict: {0}")]
    Conflict(String),

    #[error("Not Found: {0}")]
    NotFound(String),

    #[error("Request validation failed: {0}")]
    RequestValidation(String),

    #[error("Validation failed: {0}")]
    Validation(String),

    // TODO: Might make sense to move to a different error type such as AccountError
    #[error("Insufficient funds")]
    InsufficientFunds,
}
