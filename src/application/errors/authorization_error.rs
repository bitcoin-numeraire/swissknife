use thiserror::Error;
use utoipa::ToSchema;

use crate::domains::user::Permission;

#[derive(Debug, Error, ToSchema)]
pub enum AuthorizationError {
    #[error("Missing required permission: {0:?}")]
    MissingPermission(Permission),
}
