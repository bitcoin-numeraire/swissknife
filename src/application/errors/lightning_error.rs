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

    #[error("Failed to parse LNURLp metadata: {0}")]
    GenerateMetadata(String),

    #[error("Failed to send Bolt11 payment: {0}")]
    SendBolt11Payment(String),

    #[error("Failed to send payment to node: {0}")]
    SendNodeIdPayment(String),

    #[error("Failed to send LNURL payment: {0}")]
    SendLNURLPayment(String),

    #[error("Failed to retrieve payment by hash: {0}")]
    PaymentByHash(String),

    #[error("Unsupported payment format: {0}")]
    UnsupportedPaymentFormat(String),
}
