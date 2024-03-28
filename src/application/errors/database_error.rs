use thiserror::Error;

#[derive(Debug, Error)]
pub enum DatabaseError {
    #[error("Failed to parse config: {0}")]
    ParseConfig(String),

    #[error("Failed to connect to database: {0}")]
    Connect(String),

    #[error("Failed to get resource: {0}")]
    Get(String),

    #[error("Failed to list resources: {0}")]
    List(String),

    #[error("Failed to insert resource: {0}")]
    Insert(String),

    #[error("Failed to update resource: {0}")]
    Update(String),

    #[error("Failed to compute balance: {0}")]
    Balance(String),
}
