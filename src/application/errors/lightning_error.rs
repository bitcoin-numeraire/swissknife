use thiserror::Error;

#[derive(Debug, Error)]
pub enum LightningError {
    #[error("Failed to initialize logging: {0}")]
    Logging(String),

    #[error("Failed to parse mnemonic seed: {0}")]
    ParseSeed(String),

    #[error("Failed to read certificates: {0}")]
    ReadCertificates(String),

    #[error("Failed to connect to lightning node or service: {0}")]
    Connect(String),

    #[error("Failed to generate Lightning invoice: {0}")]
    Invoice(String),

    #[error("Failed to get Lightning node info: {0}")]
    NodeInfo(String),

    #[error("Failed to get LSP info: {0}")]
    LSPInfo(String),

    #[error("Failed to list LSPs: {0}")]
    ListLSPs(String),

    #[error("Failed to get list payments from Lightning node: {0}")]
    ListPayments(String),

    #[error("Failed to send Bolt11 payment: {0}")]
    SendBolt11Payment(String),

    #[error("Failed to send payment to node: {0}")]
    SendNodeIdPayment(String),

    #[error("Failed to send LNURL payment: {0}")]
    SendLNURLPayment(String),

    #[error("Failed to retrieve payment by hash: {0}")]
    PaymentByHash(String),

    #[error("Failed to close LSP channels: {0}")]
    CloseLSPChannels(String),

    #[error("Failed to pay on-chain: {0}")]
    PayOnChain(String),

    #[error("Failed to redeem on-chain: {0}")]
    RedeemOnChain(String),

    #[error("Failed to retrieve healthcheck: {0}")]
    HealthCheck(String),
}
