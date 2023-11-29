use super::{AsyncError, ConfigError, LightningError, RGBError};

#[derive(Debug)]
pub enum ApplicationError {
    Config(ConfigError),
    Async(AsyncError),
    RGB(RGBError),
    Lightning(LightningError),
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

impl From<LightningError> for ApplicationError {
    fn from(inner: LightningError) -> Self {
        ApplicationError::Lightning(inner)
    }
}
