use thiserror::Error;

#[derive(Debug, Error)]
pub enum DatabaseError {
    #[error("Failed to parse config: {0}")]
    ParseConfig(String),

    #[error("Failed to connect to database: {0}")]
    Connect(String),

    #[error("Failed to find resource: {0}")]
    Find(String),

    #[error("Failed to find all resources: {0}")]
    FindAll(String),

    #[error("Failed to find resource by statement: {0}")]
    FindByStatement(String),

    #[error("Failed to save resource: {0}")]
    Insert(String),

    #[error("Failed to update resource: {0}")]
    Update(String),

    #[error("Failed to delete resource: {0}")]
    Delete(String),

    #[error("Failed to perform transaction operation: {0}")]
    Transaction(String),
}
