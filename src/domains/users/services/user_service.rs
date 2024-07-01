use async_trait::async_trait;
use std::sync::Arc;

use tracing::{debug, trace};

use crate::{
    application::{dtos::AuthProvider, errors::ApplicationError},
    domains::users::entities::AuthUser,
    infra::auth::Authenticator,
};

use super::UserUseCases;

pub struct UserService {
    provider: AuthProvider,
    authenticator: Arc<dyn Authenticator>,
}

impl UserService {
    pub fn new(provider: AuthProvider, authenticator: Arc<dyn Authenticator>) -> Self {
        UserService {
            provider,
            authenticator,
        }
    }
}

#[async_trait]
impl UserUseCases for UserService {
    fn sign_in(&self, password: String) -> Result<String, ApplicationError> {
        trace!("Start login");

        match self.provider {
            AuthProvider::Jwt => {
                let token = self.authenticator.generate_jwt_token(&password)?;

                debug!(%token, "User logged in successfully");
                Ok(token)
            }
            _ => Err(ApplicationError::UnsupportedOperation(format!(
                "login for {} provider",
                self.provider
            ))),
        }
    }

    async fn authenticate(&self, token: &str) -> Result<AuthUser, ApplicationError> {
        trace!(%token, "Start JWT authentication");

        let user = self.authenticator.authenticate(token).await?;

        trace!(user = ?user, "JWT authentication successful");
        Ok(user)
    }

    fn provider(&self) -> AuthProvider {
        self.provider.clone()
    }
}
