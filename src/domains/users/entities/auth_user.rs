use crate::application::errors::AuthorizationError;

use super::permission::Permission;

#[derive(Clone, Debug)]
pub struct AuthUser {
    pub sub: String,
    pub permissions: Vec<Permission>,
}

impl Default for AuthUser {
    fn default() -> Self {
        Self {
            sub: "superuser".to_string(),
            permissions: Permission::all_permissions(),
        }
    }
}

impl AuthUser {
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
