use thiserror::Error;

#[derive(Debug, Error)]
pub enum WebServerError {
    #[error("Failed to parse config: {0}")]
    ParseConfig(String),

    #[error("Failed to create TCP listener: {0}")]
    Listener(String),

    #[error("Failed to serve application: {0}")]
    Serve(String),
}
