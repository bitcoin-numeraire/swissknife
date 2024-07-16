use async_trait::async_trait;
use std::sync::Arc;

use tracing::{debug, info, trace};

use crate::{
    application::{
        dtos::AuthProvider,
        entities::{AppStore, Currency},
        errors::ApplicationError,
    },
    infra::auth::Authenticator,
};

use super::{Account, AuthUseCases};

pub struct AuthService {
    provider: AuthProvider,
    authenticator: Arc<dyn Authenticator>,
    store: AppStore,
}

impl AuthService {
    pub fn new(
        provider: AuthProvider,
        authenticator: Arc<dyn Authenticator>,
        store: AppStore,
    ) -> Self {
        AuthService {
            provider,
            authenticator,
            store,
        }
    }
}

#[async_trait]
impl AuthUseCases for AuthService {
    fn sign_in(&self, password: String) -> Result<String, ApplicationError> {
        trace!("Start login");

        match self.provider {
            AuthProvider::Jwt => {
                let token = self.authenticator.generate(&password)?;

                debug!(%token, "User logged in successfully");
                Ok(token)
            }
            _ => Err(ApplicationError::UnsupportedOperation(format!(
                "Sign in not allowed (not needed) for {} provider",
                self.provider
            ))),
        }
    }

    async fn authenticate(&self, token: &str) -> Result<Account, ApplicationError> {
        trace!(%token, "Start JWT authentication");

        let claims = self.authenticator.decode(token).await?;
        trace!(?claims, "Token decoded successfully");

        let user_opt = self.store.account.find_by_sub(&claims.sub).await?;

        let mut user = match user_opt {
            Some(user) => user,
            None => {
                let mut new_user = self.store.account.insert(&claims.sub).await?;
                let wallet = self
                    .store
                    .wallet
                    .insert(new_user.id, Currency::Bitcoin)
                    .await?;

                new_user.wallet = wallet;

                info!(user_id = %new_user.id, "New user created successfully on first login");
                new_user
            }
        };
        user.permissions = claims.permissions;

        trace!(?user, "Authentication successful");
        Ok(user)
    }

    fn provider(&self) -> AuthProvider {
        self.provider.clone()
    }
}
