use thiserror::Error;

#[derive(Debug, Error)]
pub enum DataError {
    #[error("Conflict: {0}")]
    Conflict(String),

    #[error("Not Found: {0}")]
    NotFound(String),

    #[error("Unprocessable: {0}")]
    Unprocessable(String),
}
