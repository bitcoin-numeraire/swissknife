use uuid::Uuid;

use crate::application::errors::AuthorizationError;

use super::permission::Permission;

#[derive(Clone, Debug, Default)]
pub struct User {
    pub wallet_id: Uuid,
    pub permissions: Vec<Permission>,
}

impl User {
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
