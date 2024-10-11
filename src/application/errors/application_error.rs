use thiserror::Error;
use utoipa::ToSchema;

use super::{
    AuthenticationError, AuthorizationError, ConfigError, DataError, DatabaseError, LightningError,
    WebServerError,
};

#[derive(Debug, Error, ToSchema)]
pub enum ApplicationError {
    #[error(transparent)]
    Config(#[from] ConfigError),

    #[error("Lightning Error: {0}")]
    Lightning(#[from] LightningError),

    #[error("Web Server Error: {0}")]
    WebServer(#[from] WebServerError),

    #[error("Authentication Error: {0}")]
    Authentication(#[from] AuthenticationError),

    #[error("Authorization Error: {0}")]
    Authorization(#[from] AuthorizationError),

    #[error("Database Error: {0}")]
    Database(#[from] DatabaseError),

    #[error("Data Error: {0}")]
    Data(#[from] DataError),
}
