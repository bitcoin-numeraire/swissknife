use std::sync::Arc;

use async_trait::async_trait;
use bcrypt::{hash, verify, DEFAULT_COST};
use serde_bolt::bitcoin::hashes::{sha256, Hash};

use tracing::{debug, info, trace};

use crate::{
    application::{
        dtos::AuthProvider,
        entities::AppStore,
        errors::{ApplicationError, AuthenticationError, DataError},
    },
    infra::jwt::JWTAuthenticator,
};

use super::{AuthUseCases, Permission, User};

pub const PASSWORD_HASH_KEY: &str = "password_hash";

pub struct AuthService {
    jwt_authenticator: Arc<dyn JWTAuthenticator>,
    store: AppStore,
    provider: AuthProvider,
}

impl AuthService {
    pub fn new(jwt_authenticator: Arc<dyn JWTAuthenticator>, store: AppStore, provider: AuthProvider) -> Self {
        AuthService {
            jwt_authenticator,
            store,
            provider,
        }
    }
}

#[async_trait]
impl AuthUseCases for AuthService {
    async fn sign_up(&self, password: String) -> Result<String, ApplicationError> {
        trace!("Start sign up");

        if self.provider != AuthProvider::Jwt {
            return Err(AuthenticationError::UnsupportedOperation.into());
        }

        if self.store.config.find(PASSWORD_HASH_KEY).await?.is_some() {
            return Err(DataError::Conflict("Admin user already created".into()).into());
        }

        let password_hash = hash(&password, DEFAULT_COST).map_err(|e| AuthenticationError::Hash(e.to_string()))?;

        self.store
            .config
            .insert(PASSWORD_HASH_KEY, password_hash.into())
            .await?;

        let token = self
            .jwt_authenticator
            .encode("admin".to_string(), Permission::all_permissions())?;

        debug!("Admin user created successfully");
        Ok(token)
    }

    async fn sign_in(&self, password: String) -> Result<String, ApplicationError> {
        trace!("Start login");

        if self.provider != AuthProvider::Jwt {
            return Err(AuthenticationError::UnsupportedOperation.into());
        }
        match self.store.config.find(PASSWORD_HASH_KEY).await? {
            Some(password_hash) => {
                let password_hash_str = password_hash
                    .as_str()
                    .ok_or_else(|| DataError::Inconsistency("Expected string in password hash".to_string()))?;

                if !verify(password, password_hash_str).map_err(|e| AuthenticationError::Hash(e.to_string()))? {
                    return Err(AuthenticationError::InvalidCredentials.into());
                }

                let token = self
                    .jwt_authenticator
                    .encode("admin".to_string(), Permission::all_permissions())?;

                debug!("User logged in successfully");
                Ok(token)
            }
            None => Err(DataError::NotFound("Missing admin credentials".into()).into()),
        }
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
                return Err(DataError::Inconsistency("Existing API key without wallet".to_string()).into());
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
