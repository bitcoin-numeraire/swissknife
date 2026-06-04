use uuid::Uuid;

use crate::application::errors::AuthorizationError;

use super::permission::Permission;

#[derive(Clone, Debug, Default)]
pub struct User {
    pub id: String,
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn has_permission_returns_true_only_for_assigned_permissions() {
        let user = User {
            permissions: vec![Permission::ReadWallet, Permission::ReadApiKey],
            ..Default::default()
        };

        assert!(user.has_permission(Permission::ReadWallet));
        assert!(user.has_permission(Permission::ReadApiKey));
        assert!(!user.has_permission(Permission::WriteWallet));
    }

    #[test]
    fn check_permission_allows_assigned_permission() {
        let user = User {
            permissions: vec![Permission::WriteApiKey],
            ..Default::default()
        };

        assert!(user.check_permission(Permission::WriteApiKey).is_ok());
    }

    #[test]
    fn check_permission_returns_missing_permission_error() {
        let user = User {
            permissions: vec![Permission::ReadWallet],
            ..Default::default()
        };

        let err = user.check_permission(Permission::WriteWallet).unwrap_err();
        assert!(matches!(
            err,
            AuthorizationError::MissingPermission(Permission::WriteWallet)
        ));
    }
}
