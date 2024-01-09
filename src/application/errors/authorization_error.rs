use thiserror::Error;

use crate::domains::users::entities::Permission;

#[derive(Debug, Error)]
pub enum AuthorizationError {
    #[error("Missing required permission: {0:?}")]
    MissingPermission(Permission),

    #[error("Failed to parse permission: {0}")]
    ParsePermission(String),
}
