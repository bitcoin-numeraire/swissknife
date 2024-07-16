use crate::application::errors::AuthenticationError;
use crate::domains::user::{AuthClaims, Permission};
use crate::infra::auth::Authenticator;
use async_trait::async_trait;

#[derive(Clone, Debug)]
pub struct BypassAuthenticator {}

impl BypassAuthenticator {
    pub fn new() -> Self {
        Self {}
    }
}

const SUB: &str = "superuser";

#[async_trait]
impl Authenticator for BypassAuthenticator {
    fn generate(&self, _: &str) -> Result<String, AuthenticationError> {
        Err(AuthenticationError::UnsupportedOperation)
    }

    async fn decode(&self, _: &str) -> Result<AuthClaims, AuthenticationError> {
        Ok(AuthClaims {
            sub: SUB.to_string(),
            permissions: Permission::all_permissions(),
        })
    }
}
