use thiserror::Error;

use super::{
    AuthenticationError, ConfigError, DatabaseError, LightningError, RGBError, WebServerError,
};

#[derive(Debug, Error)]
pub enum ApplicationError {
    #[error(transparent)]
    Config(#[from] ConfigError),

    #[error("RGB Error: {0}")]
    RGB(#[from] RGBError),

    #[error("Lightning Error: {0}")]
    Lightning(#[from] LightningError),

    #[error("Web Server Error: {0}")]
    WebServer(#[from] WebServerError),

    #[error("Authentication Error: {0}")]
    Authentication(#[from] AuthenticationError),

    #[error("Database Error: {0}")]
    Database(#[from] DatabaseError),
}
