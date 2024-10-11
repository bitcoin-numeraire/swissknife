use std::sync::Arc;

use async_trait::async_trait;
use serde_bolt::bitcoin::hashes::{sha256, Hash};

use tracing::{debug, info, trace};

use crate::{
    application::{
        entities::AppStore,
        errors::{ApplicationError, AuthenticationError, DataError},
    },
    infra::jwt::JWTAuthenticator,
};

use super::{AuthUseCases, User};

pub struct AuthService {
    jwt_authenticator: Arc<dyn JWTAuthenticator>,
    store: AppStore,
}

impl AuthService {
    pub fn new(jwt_authenticator: Arc<dyn JWTAuthenticator>, store: AppStore) -> Self {
        AuthService {
            jwt_authenticator,
            store,
        }
    }
}

#[async_trait]
impl AuthUseCases for AuthService {
    fn sign_in(&self, password: String) -> Result<String, ApplicationError> {
        trace!("Start login");

        let token = self.jwt_authenticator.generate(&password)?;

        debug!("User logged in successfully");
        Ok(token)
    }

    async fn authenticate_jwt(&self, token: &str) -> Result<User, ApplicationError> {
        trace!("Start JWT authentication");

        let claims = self.jwt_authenticator.decode(token).await?;

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

    async fn authenticate_api_key(&self, token: Vec<u8>) -> Result<User, ApplicationError> {
        trace!("Start API Key authentication");

        let key_hash = sha256::Hash::hash(&token).to_vec();
        let api_key_opt = self.store.api_key.find_by_key_hash(key_hash).await?;

        let api_key = match api_key_opt {
            Some(api_key) => api_key,
            None => {
                return Err(AuthenticationError::InvalidCredentials.into());
            }
        };

        let wallet_opt = self.store.wallet.find_by_user_id(&api_key.user_id).await?;

        let wallet = match wallet_opt {
            Some(wallet) => wallet,
            None => {
                return Err(DataError::Inconsistency(
                    "Existing API key without wallet".to_string(),
                )
                .into());
            }
        };

        let user = User {
            id: wallet.user_id,
            wallet_id: wallet.id,
            permissions: api_key.permissions,
        };

        trace!(?user, "Authentication successful");
        Ok(user)
    }
}
