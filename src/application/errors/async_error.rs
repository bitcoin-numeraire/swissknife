use tokio::task::JoinError;

#[derive(Debug)]
pub enum AsyncError {
    TaskJoinError(String),
}

impl From<JoinError> for AsyncError {
    fn from(error: JoinError) -> Self {
        AsyncError::TaskJoinError(error.to_string())
    }
}
