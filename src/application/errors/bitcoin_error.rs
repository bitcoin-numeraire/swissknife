use thiserror::Error;
use utoipa::ToSchema;

#[derive(Debug, Error, ToSchema)]
pub enum BitcoinError {
    #[error("Failed to get bitcoin address: {0}")]
    Address(String),

    #[error("Failed to get bitcoin balance: {0}")]
    Balance(String),

    #[error("Failed to send bitcoin transaction: {0}")]
    Transaction(String),

    #[error("Failed to list bitcoin outputs: {0}")]
    Outputs(String),

    #[error("Failed to get bitcoin network: {0}")]
    Network(String),

    #[error("Unsupported bitcoin operation: {0}")]
    Unsupported(String),
}
