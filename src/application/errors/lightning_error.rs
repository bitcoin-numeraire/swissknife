use thiserror::Error;

#[derive(Debug, Error)]
pub enum LightningError {
    #[error("Failed to parse mnemonic seed: {0}")]
    ParseSeed(String),

    #[error("Failed to connect to lightning node or service: {0}")]
    Connect(String),

    #[error("Failed to generate Lightning invoice: {0}")]
    Invoice(String),

    #[error("Failed to get Lightning node info: {0}")]
    NodeInfo(String),

    #[error("Failed to get LSP info: {0}")]
    LSPInfo(String),

    #[error("Failed to get list payments from Lightning node: {0}")]
    ListPayments(String),

    #[error("Failed to register Lightning address: {0}")]
    Register(String),

    #[error("Failed to parse LNURLp metadata: {0}")]
    ParseMetadata(String),

    #[error("Failed to send Bolt11 payment: {0}")]
    SendBolt11Payment(String),
}
