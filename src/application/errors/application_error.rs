use super::{AsyncError, ConfigError, RGBError};

#[derive(Debug)]
pub enum ApplicationError {
    RGB(RGBError),
    Config(ConfigError),
    Async(AsyncError),
}

impl From<RGBError> for ApplicationError {
    fn from(inner: RGBError) -> Self {
        ApplicationError::RGB(inner)
    }
}

impl From<AsyncError> for ApplicationError {
    fn from(inner: AsyncError) -> Self {
        ApplicationError::Async(inner)
    }
}

impl From<ConfigError> for ApplicationError {
    fn from(inner: ConfigError) -> Self {
        ApplicationError::Config(inner)
    }
}
