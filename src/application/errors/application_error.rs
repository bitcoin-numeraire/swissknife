use thiserror::Error;

use super::{
    AuthenticationError, ConfigError, DatabaseError, LightningError, RGBError, WebServerError,
};

#[derive(Debug, Error)]
pub enum ApplicationError {
    #[error(transparent)]
    Config(#[from] ConfigError),

    #[error(transparent)]
    RGB(#[from] RGBError),

    #[error(transparent)]
    Lightning(#[from] LightningError),

    #[error(transparent)]
    WebServer(#[from] WebServerError),

    #[error(transparent)]
    Authentication(#[from] AuthenticationError),

    #[error(transparent)]
    Database(#[from] DatabaseError),
}
