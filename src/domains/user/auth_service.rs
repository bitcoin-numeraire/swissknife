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

        let key_hash = sha256::Hash::hash(&token).to_byte_array().to_vec();
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

#[cfg(test)]
mod tests {
    use uuid::Uuid;

    use crate::{
        application::entities::MockAppStoreBuilder,
        domains::{
            user::{ApiKey, AuthClaims},
            wallet::Wallet,
        },
        infra::jwt::MockJWTAuthenticator,
    };

    use super::*;

    fn service(jwt: MockJWTAuthenticator, store: MockAppStoreBuilder, provider: AuthProvider) -> AuthService {
        AuthService::new(Arc::new(jwt), store.build(), provider)
    }

    fn claims(sub: &str) -> AuthClaims {
        AuthClaims {
            exp: 0,
            iat: 0,
            sub: sub.to_string(),
            permissions: vec![Permission::ReadWallet],
        }
    }

    fn wallet_fixture(id: Uuid, user_id: &str) -> Wallet {
        Wallet {
            id,
            user_id: user_id.to_string(),
            ..Default::default()
        }
    }

    mod sign_up {
        use super::*;

        mod when_provider_is_not_jwt {
            use super::*;

            #[tokio::test]
            async fn returns_unsupported_operation() {
                let service = service(
                    MockJWTAuthenticator::new(),
                    MockAppStoreBuilder::new(),
                    AuthProvider::OAuth2,
                );

                let err = service.sign_up("password".to_string()).await.unwrap_err();

                assert!(matches!(
                    err,
                    ApplicationError::Authentication(AuthenticationError::UnsupportedOperation)
                ));
            }
        }

        mod when_admin_already_exists {
            use super::*;

            #[tokio::test]
            async fn returns_conflict() {
                let mut store = MockAppStoreBuilder::new();
                store
                    .config
                    .expect_find()
                    .times(1)
                    .returning(|_| Ok(Some("existing-hash".into())));

                let service = service(MockJWTAuthenticator::new(), store, AuthProvider::Jwt);

                let err = service.sign_up("password".to_string()).await.unwrap_err();

                assert!(matches!(err, ApplicationError::Data(DataError::Conflict(_))));
            }
        }

        mod when_first_admin {
            use super::*;

            #[tokio::test]
            async fn persists_hash_and_returns_token() {
                let mut store = MockAppStoreBuilder::new();
                store.config.expect_find().times(1).returning(|_| Ok(None));
                store
                    .config
                    .expect_insert()
                    .withf(|key, _| key == PASSWORD_HASH_KEY)
                    .times(1)
                    .returning(|_, _| Ok(()));

                let mut jwt = MockJWTAuthenticator::new();
                jwt.expect_encode().times(1).returning(|_, _| Ok("token".to_string()));

                let service = service(jwt, store, AuthProvider::Jwt);

                let token = service.sign_up("password".to_string()).await.unwrap();

                assert_eq!(token, "token");
            }
        }
    }

    mod sign_in {
        use super::*;

        mod when_credentials_are_missing {
            use super::*;

            #[tokio::test]
            async fn returns_not_found() {
                let mut store = MockAppStoreBuilder::new();
                store.config.expect_find().times(1).returning(|_| Ok(None));

                let service = service(MockJWTAuthenticator::new(), store, AuthProvider::Jwt);

                let err = service.sign_in("password".to_string()).await.unwrap_err();

                assert!(matches!(err, ApplicationError::Data(DataError::NotFound(_))));
            }
        }

        mod with_a_wrong_password {
            use super::*;

            #[tokio::test]
            async fn returns_invalid_credentials() {
                let stored_hash = hash("correct", 4).unwrap();

                let mut store = MockAppStoreBuilder::new();
                store
                    .config
                    .expect_find()
                    .times(1)
                    .returning(move |_| Ok(Some(stored_hash.clone().into())));

                let service = service(MockJWTAuthenticator::new(), store, AuthProvider::Jwt);

                let err = service.sign_in("wrong".to_string()).await.unwrap_err();

                assert!(matches!(
                    err,
                    ApplicationError::Authentication(AuthenticationError::InvalidCredentials)
                ));
            }
        }

        mod with_the_correct_password {
            use super::*;

            #[tokio::test]
            async fn returns_token() {
                let stored_hash = hash("correct", 4).unwrap();

                let mut store = MockAppStoreBuilder::new();
                store
                    .config
                    .expect_find()
                    .times(1)
                    .returning(move |_| Ok(Some(stored_hash.clone().into())));

                let mut jwt = MockJWTAuthenticator::new();
                jwt.expect_encode().times(1).returning(|_, _| Ok("token".to_string()));

                let service = service(jwt, store, AuthProvider::Jwt);

                let token = service.sign_in("correct".to_string()).await.unwrap();

                assert_eq!(token, "token");
            }
        }

        mod when_stored_hash_is_not_a_string {
            use super::*;

            #[tokio::test]
            async fn returns_inconsistency() {
                let mut store = MockAppStoreBuilder::new();
                store
                    .config
                    .expect_find()
                    .times(1)
                    .returning(|_| Ok(Some(serde_json::json!(42))));

                let service = service(MockJWTAuthenticator::new(), store, AuthProvider::Jwt);

                let err = service.sign_in("password".to_string()).await.unwrap_err();

                assert!(matches!(err, ApplicationError::Data(DataError::Inconsistency(_))));
            }
        }
    }

    mod authenticate_jwt {
        use super::*;

        mod when_wallet_exists {
            use super::*;

            #[tokio::test]
            async fn returns_user_for_existing_wallet() {
                let wallet_id = Uuid::new_v4();

                let mut jwt = MockJWTAuthenticator::new();
                jwt.expect_decode().times(1).returning(|_| Ok(claims("alice")));

                let mut store = MockAppStoreBuilder::new();
                store
                    .wallet
                    .expect_find_by_user_id()
                    .withf(|user_id| user_id == "alice")
                    .times(1)
                    .returning(move |user_id| Ok(Some(wallet_fixture(wallet_id, user_id))));

                let service = service(jwt, store, AuthProvider::Jwt);

                let user = service.authenticate_jwt("token").await.unwrap();

                assert_eq!(user.id, "alice");
                assert_eq!(user.wallet_id, wallet_id);
            }
        }

        mod when_wallet_is_missing {
            use super::*;

            #[tokio::test]
            async fn provisions_a_wallet_on_first_login() {
                let mut jwt = MockJWTAuthenticator::new();
                jwt.expect_decode().times(1).returning(|_| Ok(claims("alice")));

                let mut store = MockAppStoreBuilder::new();
                store.wallet.expect_find_by_user_id().times(1).returning(|_| Ok(None));
                store
                    .wallet
                    .expect_insert()
                    .withf(|user_id| user_id == "alice")
                    .times(1)
                    .returning(|user_id| Ok(wallet_fixture(Uuid::new_v4(), user_id)));

                let service = service(jwt, store, AuthProvider::Jwt);

                let user = service.authenticate_jwt("token").await.unwrap();

                assert_eq!(user.id, "alice");
            }
        }

        mod when_token_is_invalid {
            use super::*;

            #[tokio::test]
            async fn propagates_authentication_error() {
                let mut jwt = MockJWTAuthenticator::new();
                jwt.expect_decode()
                    .times(1)
                    .returning(|_| Err(AuthenticationError::InvalidCredentials));

                let service = service(jwt, MockAppStoreBuilder::new(), AuthProvider::Jwt);

                let err = service.authenticate_jwt("token").await.unwrap_err();

                assert!(matches!(err, ApplicationError::Authentication(_)));
            }
        }
    }

    mod authenticate_api_key {
        use super::*;

        mod when_key_is_unknown {
            use super::*;

            #[tokio::test]
            async fn returns_invalid_credentials() {
                let mut store = MockAppStoreBuilder::new();
                store.api_key.expect_find_by_key_hash().times(1).returning(|_| Ok(None));

                let service = service(MockJWTAuthenticator::new(), store, AuthProvider::Jwt);

                let err = service.authenticate_api_key(vec![1, 2, 3]).await.unwrap_err();

                assert!(matches!(
                    err,
                    ApplicationError::Authentication(AuthenticationError::InvalidCredentials)
                ));
            }
        }

        mod when_key_and_wallet_exist {
            use super::*;

            #[tokio::test]
            async fn returns_user_with_api_key_permissions() {
                let wallet_id = Uuid::new_v4();

                let mut store = MockAppStoreBuilder::new();
                store.api_key.expect_find_by_key_hash().times(1).returning(|_| {
                    Ok(Some(ApiKey {
                        user_id: "alice".to_string(),
                        permissions: vec![Permission::ReadWallet],
                        ..Default::default()
                    }))
                });
                store
                    .wallet
                    .expect_find_by_user_id()
                    .times(1)
                    .returning(move |user_id| Ok(Some(wallet_fixture(wallet_id, user_id))));

                let service = service(MockJWTAuthenticator::new(), store, AuthProvider::Jwt);

                let user = service.authenticate_api_key(vec![1, 2, 3]).await.unwrap();

                assert_eq!(user.wallet_id, wallet_id);
                assert_eq!(user.permissions, vec![Permission::ReadWallet]);
            }
        }

        mod when_key_exists_without_wallet {
            use super::*;

            #[tokio::test]
            async fn returns_inconsistency() {
                let mut store = MockAppStoreBuilder::new();
                store.api_key.expect_find_by_key_hash().times(1).returning(|_| {
                    Ok(Some(ApiKey {
                        user_id: "alice".to_string(),
                        ..Default::default()
                    }))
                });
                store.wallet.expect_find_by_user_id().times(1).returning(|_| Ok(None));

                let service = service(MockJWTAuthenticator::new(), store, AuthProvider::Jwt);

                let err = service.authenticate_api_key(vec![1, 2, 3]).await.unwrap_err();

                assert!(matches!(err, ApplicationError::Data(DataError::Inconsistency(_))));
            }
        }
    }
}
