use thiserror::Error;

#[derive(Debug, Error)]
pub enum LightningError {
    #[error("Failed to initialize logging: {0}")]
    Logging(String),

    #[error("Failed to parse config: {0}")]
    ParseConfig(String),

    #[error("Failed to parse mnemonic seed: {0}")]
    ParseSeed(String),

    #[error("Failed to read certificates: {0}")]
    ReadCertificates(String),

    #[error("Failed to parse TLS config: {0}")]
    TLSConfig(String),

    #[error("Failed to connect to Lightning node: {0}")]
    Connect(String),

    #[error("Failed to connect to Lightning node websocket server: {0}")]
    ConnectWebsocket(String),

    #[error("Failed to disconnect from Lightning node: {0}")]
    Disconnect(String),

    #[error("Lightning event listener failure: {0}")]
    Listener(String),

    #[error("Failed to generate Lightning invoice: {0}")]
    Invoice(String),

    #[error("Failed to get Lightning node info: {0}")]
    NodeInfo(String),

    #[error("Failed to get LSP info: {0}")]
    LSPInfo(String),

    #[error("Failed to list LSPs: {0}")]
    ListLSPs(String),

    #[error("Failed to send payment: {0}")]
    Pay(String),

    #[error("Failed to get invoice by hash: {0}")]
    InvoiceByHash(String),

    #[error("Failed to close LSP channels: {0}")]
    CloseLSPChannels(String),

    #[error("Failed to pay on-chain: {0}")]
    PayOnChain(String),

    #[error("Failed to redeem on-chain: {0}")]
    RedeemOnChain(String),

    #[error("Failed to connect to LSP: {0}")]
    ConnectLSP(String),

    #[error("Failed to retrieve healthcheck: {0}")]
    HealthCheck(String),

    #[error("Failed to sync node: {0}")]
    Sync(String),

    #[error("Failed to backup node state: {0}")]
    Backup(String),

    #[error("Failed to sign message: {0}")]
    SignMessage(String),

    #[error("Failed to check message signature: {0}")]
    CheckMessage(String),
}
