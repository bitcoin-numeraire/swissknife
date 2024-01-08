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
