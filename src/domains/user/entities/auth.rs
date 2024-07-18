use super::permission::Permission;

#[derive(Clone, Debug)]
pub struct AuthClaims {
    pub sub: String,
    pub permissions: Vec<Permission>,
}
