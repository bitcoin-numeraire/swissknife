use chrono::{DateTime, Utc};
use uuid::Uuid;

use crate::{
    application::errors::AuthorizationError,
    domains::{ln_address::LnAddress, wallet::Wallet},
};

use super::permission::Permission;

#[derive(Clone, Debug, Default)]
pub struct Account {
    pub id: Uuid,
    pub wallet: Wallet,
    pub ln_address: Option<LnAddress>,
    pub permissions: Vec<Permission>,
    pub created_at: DateTime<Utc>,
    pub updated_at: Option<DateTime<Utc>>,
}

impl Account {
    pub fn has_permission(&self, permission: Permission) -> bool {
        self.permissions.contains(&permission)
    }

    pub fn check_permission(&self, permission: Permission) -> Result<(), AuthorizationError> {
        if !self.has_permission(permission.clone()) {
            return Err(AuthorizationError::MissingPermission(permission));
        }

        Ok(())
    }
}
