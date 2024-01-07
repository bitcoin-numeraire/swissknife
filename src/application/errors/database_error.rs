use thiserror::Error;

#[derive(Debug, Error)]
pub enum DatabaseError {
    #[error("Failed to parse config: {0}")]
    ParseConfig(String),

    #[error("Failed to connect to database: {0}")]
    Connect(String),
}
