use crate::application::errors::AuthenticationError;
use crate::domains::user::{AuthUser, Permission};
use crate::infra::auth::Authenticator;
use async_trait::async_trait;

#[derive(Clone, Debug)]
pub struct BypassAuthenticator {}

impl BypassAuthenticator {
    pub fn new() -> Self {
        Self {}
    }
}

const USERNAME: &str = "superuser";

#[async_trait]
impl Authenticator for BypassAuthenticator {
    fn generate_jwt_token(&self, _: &str) -> Result<String, AuthenticationError> {
        Err(AuthenticationError::UnsupportedOperation)
    }

    async fn authenticate(&self, _: &str) -> Result<AuthUser, AuthenticationError> {
        Ok(AuthUser {
            sub: USERNAME.to_string(),
            permissions: Permission::all_permissions(),
        })
    }
}
