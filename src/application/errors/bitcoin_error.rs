use thiserror::Error;
use utoipa::ToSchema;

#[derive(Debug, Error, ToSchema)]
pub enum BitcoinError {
    #[error("Failed to get bitcoin address: {0}")]
    Address(String),

    #[error("Unsupported address type: {0}")]
    AddressType(String),

    #[error("Failed to send bitcoin transaction: {0}")]
    Transaction(String),

    #[error("Failed to get bitcoin output: {0}")]
    GetOutput(String),

    #[error("Unsupported bitcoin operation: {0}")]
    Unsupported(String),
}
