use async_trait::async_trait;
use std::sync::Arc;

use tracing::{debug, info, trace};

use crate::{
    application::{dtos::AuthProvider, entities::AppStore, errors::ApplicationError},
    infra::auth::Authenticator,
};

use super::{AuthUseCases, User};

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

    async fn authenticate(&self, token: &str) -> Result<User, ApplicationError> {
        trace!(%token, "Start JWT authentication");

        let claims = self.authenticator.decode(token).await?;
        trace!(?claims, "Token decoded successfully");

        let wallet_opt = self.store.wallet.find_by_user_id(&claims.sub).await?;

        let wallet = match wallet_opt {
            Some(wallet) => wallet,
            None => {
                let wallet = self.store.wallet.insert(&claims.sub).await?;

                info!(wallet_id = %wallet.id, user_id = %wallet.user_id, "New user created successfully on first login");
                wallet
            }
        };

        let user = User {
            id: claims.sub,
            wallet_id: wallet.id,
            permissions: claims.permissions,
        };

        trace!(?user, "Authentication successful");
        Ok(user)
    }

    fn provider(&self) -> AuthProvider {
        self.provider.clone()
    }
}
