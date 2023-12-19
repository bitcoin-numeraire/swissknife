use super::{
    AsyncError, AuthenticationError, ConfigError, LightningError, RGBError, WebServerError,
};

#[derive(Debug)]
pub enum ApplicationError {
    Config(ConfigError),
    Async(AsyncError),
    RGB(RGBError),
    Lightning(LightningError),
    WebServer(WebServerError),
    Authentication(AuthenticationError),
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

impl From<RGBError> for ApplicationError {
    fn from(inner: RGBError) -> Self {
        ApplicationError::RGB(inner)
    }
}

impl From<LightningError> for ApplicationError {
    fn from(inner: LightningError) -> Self {
        ApplicationError::Lightning(inner)
    }
}

impl From<WebServerError> for ApplicationError {
    fn from(inner: WebServerError) -> Self {
        ApplicationError::WebServer(inner)
    }
}

impl From<AuthenticationError> for ApplicationError {
    fn from(inner: AuthenticationError) -> Self {
        ApplicationError::Authentication(inner)
    }
}
