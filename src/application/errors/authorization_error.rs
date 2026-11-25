use thiserror::Error;

use crate::domains::account::Permission;

#[derive(Debug, Error)]
pub enum AuthorizationError {
    #[error("Wallet credential does not allow this operation")]
    WalletCapability,
    #[error("Missing required permission: {0:?}")]
    MissingPermission(Permission),
}
