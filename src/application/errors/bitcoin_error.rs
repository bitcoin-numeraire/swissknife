use thiserror::Error;
use utoipa::ToSchema;

#[derive(Debug, Error, ToSchema)]
pub enum BitcoinError {
    #[error("Failed to get bitcoin address: {0}")]
    Address(String),

    #[error("Unsupported address type: {0}")]
    AddressType(String),

    #[error("Failed to prepare/fund bitcoin transaction: {0}")]
    PrepareTransaction(String),

    #[error("Failed to parse bitcoin PSBT: {0}")]
    ParsePsbt(String),

    #[error("Failed to sign and send bitcoin transaction: {0}")]
    FinalizeTransaction(String),

    #[error("Failed to broadcast bitcoin transaction: {0}")]
    BroadcastTransaction(String),

    #[error("Failed to release prepared bitcoin transaction: {0}")]
    ReleaseTransaction(String),

    #[error("Failed to get bitcoin output: {0}")]
    GetOutput(String),

    #[error("Failed to get bitcoin transaction: {0}")]
    GetTransaction(String),

    #[error("Failed to synchronize bitcoin transactions: {0}")]
    Synchronize(String),

    #[error("Unsupported bitcoin operation: {0}")]
    Unsupported(String),
}
