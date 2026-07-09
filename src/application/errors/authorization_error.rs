use thiserror::Error;

use crate::domains::account::Permission;

#[derive(Debug, Error)]
pub enum AuthorizationError {
    #[error("Missing required permission: {0:?}")]
    MissingPermission(Permission),
}
