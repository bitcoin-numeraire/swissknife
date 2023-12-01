use tokio::task::JoinError;

#[derive(Debug)]
pub enum AsyncError {
    TaskJoin(String),
}

impl From<JoinError> for AsyncError {
    fn from(error: JoinError) -> Self {
        AsyncError::TaskJoin(error.to_string())
    }
}
