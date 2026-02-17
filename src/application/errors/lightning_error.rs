use thiserror::Error;
use utoipa::ToSchema;

#[derive(Debug, Error, ToSchema)]
pub enum LightningError {
    #[error("Failed to parse config: {0}")]
    ParseConfig(String),

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

    #[error("Failed to send payment: {0}")]
    Pay(String),

    #[error("Failed to cancel invoice: {0}")]
    CancelInvoice(String),

    #[error("Unexpected stream payload: {0}")]
    UnexpectedStreamPayload(String),

    #[error("Failed to get invoice by hash: {0}")]
    InvoiceByHash(String),

    #[error("Failed to get payment by hash: {0}")]
    PaymentByHash(String),

    #[error("Failed to retrieve healthcheck: {0}")]
    HealthCheck(String),
}
