use std::sync::Arc;

use async_trait::async_trait;
use bcrypt::{hash, verify, DEFAULT_COST};
use serde_bolt::bitcoin::hashes::{sha256, Hash};
use tokio::sync::OnceCell;

use tracing::{debug, trace};

use crate::{
    application::{
        composition::AppStore,
        composition::AuthProvider,
        errors::{ApplicationError, AuthenticationError, DataError},
    },
    domains::bitcoin::BtcNetwork,
    infra::jwt::JWTAuthenticator,
};

use super::{AuthUseCases, Permission, User};

pub const PASSWORD_HASH_KEY: &str = "password_hash";
const BOOTSTRAP_ADMIN_SUBJECT: &str = "admin";

pub struct AuthService {
    jwt_authenticator: Arc<dyn JWTAuthenticator>,
    store: AppStore,
    provider: AuthProvider,
    network: BtcNetwork,
    active_asset_id: OnceCell<uuid::Uuid>,
}

impl AuthService {
    pub fn new(
        jwt_authenticator: Arc<dyn JWTAuthenticator>,
        store: AppStore,
        provider: AuthProvider,
        network: BtcNetwork,
    ) -> Self {
        AuthService {
            jwt_authenticator,
            store,
            provider,
            network,
            active_asset_id: OnceCell::new(),
        }
    }

    async fn active_asset_id(&self) -> Result<uuid::Uuid, ApplicationError> {
        Ok(*self
            .active_asset_id
            .get_or_try_init(|| async {
                let asset = self
                    .store
                    .asset
                    .find_native_btc_by_network(self.network)
                    .await?
                    .ok_or_else(|| {
                        DataError::Inconsistency(format!(
                            "Missing native BTC asset for active network {}",
                            self.network
                        ))
                    })?;

                Ok::<_, ApplicationError>(asset.id)
            })
            .await?)
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

        let permissions = Permission::all_permissions();
        let account = self
            .store
            .account
            .upsert(self.provider, BOOTSTRAP_ADMIN_SUBJECT, None, &permissions)
            .await?;

        let token = self.jwt_authenticator.encode(account)?;

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

                let account = match self
                    .store
                    .account
                    .find_by_identity(self.provider, BOOTSTRAP_ADMIN_SUBJECT)
                    .await?
                {
                    Some(account) => account,
                    None => {
                        self.store
                            .account
                            .upsert(self.provider, BOOTSTRAP_ADMIN_SUBJECT, None, &[])
                            .await?
                    }
                };

                let token = self.jwt_authenticator.encode(account)?;

                debug!("User logged in successfully");
                Ok(token)
            }
            None => Err(DataError::NotFound("Missing admin credentials".into()).into()),
        }
    }

    async fn change_password(&self, current_password: String, new_password: String) -> Result<(), ApplicationError> {
        trace!("Start password change");

        if self.provider != AuthProvider::Jwt {
            return Err(AuthenticationError::UnsupportedOperation.into());
        }

        let password_hash = self
            .store
            .config
            .find(PASSWORD_HASH_KEY)
            .await?
            .ok_or_else(|| DataError::NotFound("Missing admin credentials".into()))?;
        let password_hash_str = password_hash
            .as_str()
            .ok_or_else(|| DataError::Inconsistency("Expected string in password hash".to_string()))?;

        if !verify(&current_password, password_hash_str).map_err(|e| AuthenticationError::Hash(e.to_string()))? {
            return Err(DataError::Validation("Current password is incorrect".to_string()).into());
        }

        let new_password_hash =
            hash(&new_password, DEFAULT_COST).map_err(|e| AuthenticationError::Hash(e.to_string()))?;

        self.store
            .config
            .upsert(PASSWORD_HASH_KEY, new_password_hash.into())
            .await?;

        debug!("Admin password changed successfully");
        Ok(())
    }

    async fn authenticate_jwt(&self, token: &str) -> Result<User, ApplicationError> {
        trace!("Start JWT authentication");

        let claims = self.jwt_authenticator.decode(token).await?;
        let account = match self.store.account.find_by_identity(self.provider, &claims.sub).await? {
            Some(account) => account,
            None => self.store.account.upsert(self.provider, &claims.sub, None, &[]).await?,
        };
        let permissions = if self.provider == AuthProvider::Jwt {
            account.permissions.unwrap_or_default()
        } else {
            // OAuth2 claims are authoritative for request-time permissions; DB
            // account permissions are only used by local JWT identities.
            claims.permissions
        };

        let asset_id = self.active_asset_id().await?;
        let wallet = match self
            .store
            .wallet
            .find_by_account_and_asset(account.id, asset_id)
            .await?
        {
            Some(wallet) => wallet,
            None => self.store.wallet.upsert(account.id, asset_id).await?,
        };

        trace!(
            wallet_id = %wallet.id,
            account_id = %account.id,
            "Account active asset wallet available after authentication"
        );

        let user = User {
            account_id: account.id,
            permissions,
        };

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

        let user = User {
            account_id: api_key.account_id,
            permissions: api_key.permissions,
        };

        Ok(user)
    }
}

#[cfg(test)]
mod tests {
    use chrono::Utc;
    use uuid::Uuid;

    use crate::{
        application::composition::MockAppStoreBuilder,
        domains::{
            asset::{Asset, Protocol, NATIVE_ASSET_REF},
            bitcoin::BtcNetwork,
            user::{Account, ApiKey, AuthClaims, AuthIdentity},
            wallet::Wallet,
        },
        infra::jwt::MockJWTAuthenticator,
    };

    use super::*;

    fn service(jwt: MockJWTAuthenticator, store: MockAppStoreBuilder, provider: AuthProvider) -> AuthService {
        AuthService::new(Arc::new(jwt), store.build(), provider, BtcNetwork::Regtest)
    }

    fn claims(sub: &str) -> AuthClaims {
        AuthClaims {
            exp: 0,
            iat: 0,
            sub: sub.to_string(),
            permissions: vec![Permission::ReadWallet],
        }
    }

    fn asset_fixture(id: Uuid) -> Asset {
        Asset {
            id,
            code: "BTC".to_string(),
            name: Some("Bitcoin regtest".to_string()),
            protocol: Protocol::Bitcoin,
            network: BtcNetwork::Regtest,
            asset_ref: NATIVE_ASSET_REF.to_string(),
            display_ticker: "rBTC".to_string(),
            decimals: 11,
            created_at: chrono::Utc::now(),
            updated_at: None,
        }
    }

    fn wallet_fixture(id: Uuid, account_id: Uuid, asset_id: Uuid) -> Wallet {
        Wallet {
            id,
            account_id,
            asset_id,
            ..Default::default()
        }
    }

    fn account_fixture(id: Uuid, provider: AuthProvider, subject: &str, permissions: Vec<Permission>) -> Account {
        Account {
            id,
            display_name: None,
            identity: Some(AuthIdentity {
                id: Uuid::new_v4(),
                provider,
                subject: subject.to_string(),
                created_at: Utc::now(),
            }),
            permissions: Some(permissions),
            preferences: None,
            created_at: Utc::now(),
            updated_at: None,
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
                let account_id = Uuid::new_v4();
                let permissions = Permission::all_permissions();
                let mut store = MockAppStoreBuilder::new();
                store.config.expect_find().times(1).returning(|_| Ok(None));
                store
                    .config
                    .expect_insert()
                    .withf(|key, _| key == PASSWORD_HASH_KEY)
                    .times(1)
                    .returning(|_, _| Ok(()));
                store
                    .account
                    .expect_upsert()
                    .withf(|provider, subject, display_name, granted| {
                        *provider == AuthProvider::Jwt
                            && subject == "admin"
                            && display_name.is_none()
                            && granted == Permission::all_permissions().as_slice()
                    })
                    .times(1)
                    .returning(move |provider, subject, _, permissions| {
                        Ok(account_fixture(account_id, provider, subject, permissions.to_vec()))
                    });

                let mut jwt = MockJWTAuthenticator::new();
                let expected_permissions = permissions.clone();
                jwt.expect_encode()
                    .withf(move |account| {
                        account
                            .identity
                            .as_ref()
                            .is_some_and(|identity| identity.subject == "admin")
                            && account.permissions.as_ref() == Some(&expected_permissions)
                    })
                    .times(1)
                    .returning(|_| Ok("token".to_string()));

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
                let account_id = Uuid::new_v4();
                let stored_hash = hash("correct", 4).unwrap();

                let mut store = MockAppStoreBuilder::new();
                store
                    .config
                    .expect_find()
                    .times(1)
                    .returning(move |_| Ok(Some(stored_hash.clone().into())));
                store
                    .account
                    .expect_find_by_identity()
                    .withf(|provider, subject| *provider == AuthProvider::Jwt && subject == "admin")
                    .times(1)
                    .returning(|_, _| Ok(None));
                store
                    .account
                    .expect_upsert()
                    .withf(|provider, subject, display_name, granted| {
                        *provider == AuthProvider::Jwt
                            && subject == "admin"
                            && display_name.is_none()
                            && granted.is_empty()
                    })
                    .times(1)
                    .returning(move |provider, subject, _, _| {
                        Ok(account_fixture(
                            account_id,
                            provider,
                            subject,
                            vec![Permission::ReadWallet],
                        ))
                    });

                let mut jwt = MockJWTAuthenticator::new();
                jwt.expect_encode()
                    .withf(|account| {
                        account
                            .identity
                            .as_ref()
                            .is_some_and(|identity| identity.subject == "admin")
                            && account.permissions.as_deref() == Some(&[Permission::ReadWallet][..])
                    })
                    .times(1)
                    .returning(|_| Ok("token".to_string()));

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

    mod change_password {
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

                let err = service
                    .change_password("current".to_string(), "new".to_string())
                    .await
                    .unwrap_err();

                assert!(matches!(
                    err,
                    ApplicationError::Authentication(AuthenticationError::UnsupportedOperation)
                ));
            }
        }

        mod when_credentials_are_missing {
            use super::*;

            #[tokio::test]
            async fn returns_not_found() {
                let mut store = MockAppStoreBuilder::new();
                store.config.expect_find().times(1).returning(|_| Ok(None));

                let service = service(MockJWTAuthenticator::new(), store, AuthProvider::Jwt);

                let err = service
                    .change_password("current".to_string(), "new".to_string())
                    .await
                    .unwrap_err();

                assert!(matches!(err, ApplicationError::Data(DataError::NotFound(_))));
            }
        }

        mod with_a_wrong_current_password {
            use super::*;

            #[tokio::test]
            async fn returns_validation_error() {
                let stored_hash = hash("correct", 4).unwrap();

                let mut store = MockAppStoreBuilder::new();
                store
                    .config
                    .expect_find()
                    .times(1)
                    .returning(move |_| Ok(Some(stored_hash.clone().into())));

                let service = service(MockJWTAuthenticator::new(), store, AuthProvider::Jwt);

                let err = service
                    .change_password("wrong".to_string(), "new".to_string())
                    .await
                    .unwrap_err();

                assert!(matches!(err, ApplicationError::Data(DataError::Validation(_))));
            }
        }

        mod with_the_correct_current_password {
            use super::*;

            #[tokio::test]
            async fn persists_the_new_password_hash() {
                let stored_hash = hash("current", 4).unwrap();

                let mut store = MockAppStoreBuilder::new();
                store
                    .config
                    .expect_find()
                    .times(1)
                    .returning(move |_| Ok(Some(stored_hash.clone().into())));
                store
                    .config
                    .expect_upsert()
                    .withf(|key, value| {
                        key == PASSWORD_HASH_KEY
                            && value.as_str().is_some_and(|hash| verify("new", hash).unwrap_or(false))
                    })
                    .times(1)
                    .returning(|_, _| Ok(()));

                let service = service(MockJWTAuthenticator::new(), store, AuthProvider::Jwt);

                service
                    .change_password("current".to_string(), "new".to_string())
                    .await
                    .unwrap();
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

                let err = service
                    .change_password("current".to_string(), "new".to_string())
                    .await
                    .unwrap_err();

                assert!(matches!(err, ApplicationError::Data(DataError::Inconsistency(_))));
            }
        }
    }

    mod authenticate_jwt {
        use super::*;

        mod when_token_is_valid {
            use super::*;

            #[tokio::test]
            async fn ensures_the_active_asset_wallet() {
                let wallet_id = Uuid::new_v4();
                let account_id = Uuid::new_v4();
                let asset_id = Uuid::new_v4();

                let mut jwt = MockJWTAuthenticator::new();
                jwt.expect_decode().times(1).returning(|_| Ok(claims("alice")));

                let mut store = MockAppStoreBuilder::new();
                store
                    .account
                    .expect_find_by_identity()
                    .withf(|provider, subject| *provider == AuthProvider::Jwt && subject == "alice")
                    .times(1)
                    .returning(|_, _| Ok(None));
                store
                    .account
                    .expect_upsert()
                    .withf(|provider, subject, display_name, granted| {
                        *provider == AuthProvider::Jwt
                            && subject == "alice"
                            && display_name.is_none()
                            && granted.is_empty()
                    })
                    .times(1)
                    .returning(move |provider, subject, _, _| {
                        Ok(account_fixture(
                            account_id,
                            provider,
                            subject,
                            vec![Permission::ReadApiKey],
                        ))
                    });
                store
                    .wallet
                    .expect_find_by_account_and_asset()
                    .withf(move |account, asset| *account == account_id && *asset == asset_id)
                    .times(1)
                    .returning(|_, _| Ok(None));
                store
                    .wallet
                    .expect_upsert()
                    .withf(move |account, asset| *account == account_id && *asset == asset_id)
                    .times(1)
                    .returning(move |account, asset| Ok(wallet_fixture(wallet_id, account, asset)));
                store
                    .asset
                    .expect_find_native_btc_by_network()
                    .withf(|network| *network == BtcNetwork::Regtest)
                    .times(1)
                    .returning(move |_| Ok(Some(asset_fixture(asset_id))));

                let service = service(jwt, store, AuthProvider::Jwt);

                let user = service.authenticate_jwt("token").await.unwrap();

                assert_eq!(user.account_id, account_id);
                assert_eq!(user.permissions, vec![Permission::ReadApiKey]);
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

        mod when_key_is_valid {
            use super::*;

            #[tokio::test]
            async fn returns_user_with_api_key_permissions() {
                let account_id = Uuid::new_v4();

                let mut store = MockAppStoreBuilder::new();
                store.api_key.expect_find_by_key_hash().times(1).returning(move |_| {
                    Ok(Some(ApiKey {
                        account_id,
                        permissions: vec![Permission::ReadWallet],
                        ..Default::default()
                    }))
                });

                let service = service(MockJWTAuthenticator::new(), store, AuthProvider::Jwt);

                let user = service.authenticate_api_key(vec![1, 2, 3]).await.unwrap();

                assert_eq!(user.account_id, account_id);
                assert_eq!(user.permissions, vec![Permission::ReadWallet]);
            }
        }
    }
}
