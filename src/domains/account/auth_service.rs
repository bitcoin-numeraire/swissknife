use std::{collections::HashMap, sync::Arc, time::Instant};

use async_trait::async_trait;
use base64::{engine::general_purpose::URL_SAFE_NO_PAD, Engine};
use chrono::{Duration, Utc};
use serde_bolt::bitcoin::hashes::{sha256, Hash};
use swissknife_types::{LocalLogin, LocalLoginReset};
use tokio::sync::{Mutex, OnceCell};
use uuid::Uuid;

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

use super::{password::PasswordService, AuthUseCases, LocalCredential, Permission, User, LOCAL_AUTH_INITIALIZED_KEY};

pub struct AuthService {
    jwt_authenticator: Arc<dyn JWTAuthenticator>,
    store: AppStore,
    provider: AuthProvider,
    network: BtcNetwork,
    active_asset_id: OnceCell<uuid::Uuid>,
    passwords: PasswordService,
    login_attempts: Mutex<HashMap<String, Vec<Instant>>>,
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
            passwords: PasswordService::new(),
            login_attempts: Mutex::new(HashMap::new()),
        }
    }

    async fn limit_login(&self, username: &str) -> Result<(), ApplicationError> {
        let mut attempts = self.login_attempts.lock().await;
        let now = Instant::now();
        attempts.retain(|_, times| {
            times.retain(|time| now.duration_since(*time).as_secs() < 60);
            !times.is_empty()
        });
        if !attempts.contains_key(username) && attempts.len() >= 4096 {
            return Err(AuthenticationError::RateLimited.into());
        }
        let times = attempts.entry(username.to_string()).or_default();
        if times.len() >= 10 {
            return Err(AuthenticationError::RateLimited.into());
        }
        times.push(now);
        Ok(())
    }

    fn require_local(&self) -> Result<(), ApplicationError> {
        if self.provider != AuthProvider::Jwt {
            return Err(AuthenticationError::UnsupportedOperation.into());
        }
        Ok(())
    }

    async fn credential(&self, account_id: Uuid) -> Result<LocalCredential, ApplicationError> {
        self.require_local()?;
        self.store
            .local_credential
            .find(account_id)
            .await?
            .ok_or_else(|| DataError::NotFound("Local login not found".into()).into())
    }

    fn reset_grant() -> (LocalLoginReset, String) {
        let bytes: [u8; 32] = rand::random();
        let code = URL_SAFE_NO_PAD.encode(bytes);
        let hash = Self::reset_hash(&code);
        (
            LocalLoginReset {
                code,
                expires_at: Utc::now() + Duration::minutes(30),
            },
            hash,
        )
    }

    fn reset_hash(code: &str) -> String {
        sha256::Hash::hash(code.as_bytes()).to_string()
    }

    async fn token(&self, account_id: Uuid, revision: Uuid) -> Result<String, ApplicationError> {
        let account = self
            .store
            .account
            .find(account_id)
            .await?
            .ok_or(AuthenticationError::InvalidCredentials)?;
        Ok(self.jwt_authenticator.encode(account, revision)?)
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
        self.require_local()?;
        if self.store.config.find(LOCAL_AUTH_INITIALIZED_KEY).await?.is_some() {
            return Err(DataError::Conflict("Owner setup is already complete".into()).into());
        }
        PasswordService::validate(&password)?;
        let hash = self.passwords.hash(password).await?;
        let account_id = self
            .store
            .local_credential
            .bootstrap(hash, Permission::all_permissions())
            .await?;
        let credential = self.credential(account_id).await?;
        let token = self.token(account_id, credential.revision).await?;
        debug!(%account_id, "Local owner created");
        Ok(token)
    }

    async fn sign_in(&self, username: String, password: String) -> Result<String, ApplicationError> {
        self.require_local()?;
        if username.len() > 1024 {
            return Err(AuthenticationError::InvalidCredentials.into());
        }
        let normalized = username.trim().to_ascii_lowercase();
        self.limit_login(&normalized).await?;
        // Preserve exact legacy subjects; new handles also accept their normalized spelling.
        let credential = match self.store.local_credential.find_by_subject(&username).await? {
            Some(credential) => Some(credential),
            None if normalized != username => self.store.local_credential.find_by_subject(&normalized).await?,
            None => None,
        };
        let hash = credential
            .as_ref()
            .filter(|c| c.enabled)
            .and_then(|c| c.password_hash.clone());
        let needs_upgrade = hash.as_ref().is_some_and(|h| h.starts_with("$2"));
        let valid = self.passwords.verify(password.clone(), hash).await?;
        let mut credential = credential
            .filter(|c| c.enabled && valid)
            .ok_or(AuthenticationError::InvalidCredentials)?;
        let account_id = credential.account_id;
        let revision = credential.revision;
        if needs_upgrade {
            credential.password_hash = Some(self.passwords.hash(password).await?);
            // Rehashing the same password preserves sessions, but cannot overwrite a concurrent reset.
            self.store.local_credential.replace(credential, revision).await?;
        }
        let token = self.token(account_id, revision).await?;
        self.login_attempts.lock().await.remove(&normalized);
        debug!(%account_id, "Local user signed in");
        Ok(token)
    }

    async fn change_password(
        &self,
        account_id: Uuid,
        current_password: String,
        new_password: String,
    ) -> Result<(), ApplicationError> {
        let mut credential = self.credential(account_id).await?;
        if !credential.enabled
            || !self
                .passwords
                .verify(current_password, credential.password_hash.clone())
                .await?
        {
            return Err(DataError::Validation("Current password is incorrect".into()).into());
        }
        PasswordService::validate(&new_password)?;
        let expected = credential.revision;
        credential.password_hash = Some(self.passwords.hash(new_password).await?);
        credential.revision = Uuid::new_v4();
        credential.reset_hash = None;
        credential.reset_expires_at = None;
        self.store.local_credential.replace(credential, expected).await?;
        debug!(%account_id, "Local password changed; sessions revoked");
        Ok(())
    }

    async fn get_local_login(&self, account_id: Uuid) -> Result<Option<LocalLogin>, ApplicationError> {
        self.require_local()?;
        Ok(self.store.local_credential.find(account_id).await?.map(|c| LocalLogin {
            username: c.subject,
            enabled: c.enabled,
            password_set: c.password_hash.is_some(),
            reset_expires_at: c.reset_expires_at,
        }))
    }

    async fn create_local_login(
        &self,
        account_id: Uuid,
        username: String,
    ) -> Result<LocalLoginReset, ApplicationError> {
        self.require_local()?;
        let account = self
            .store
            .account
            .find(account_id)
            .await?
            .ok_or_else(|| DataError::NotFound("Account not found".into()))?;
        let username = if let Some(identity) = account.identity {
            if identity.provider != AuthProvider::Jwt || identity.subject != username {
                return Err(DataError::Conflict("Use the account's existing JWT subject".into()).into());
            }
            identity.subject
        } else {
            let username = username.trim().to_ascii_lowercase();
            if !(3..=64).contains(&username.len())
                || !username
                    .bytes()
                    .all(|c| c.is_ascii_alphanumeric() || b"._-".contains(&c))
            {
                return Err(DataError::Validation(
                    "Username must contain 3 to 64 ASCII letters, digits, dots, underscores, or hyphens".into(),
                )
                .into());
            }
            username
        };
        let (grant, hash) = Self::reset_grant();
        self.store
            .local_credential
            .create(account_id, username, hash, grant.expires_at)
            .await?;
        debug!(%account_id, "Local login created");
        Ok(grant)
    }

    async fn update_local_login(&self, actor: User, account_id: Uuid, enabled: bool) -> Result<(), ApplicationError> {
        actor.check_permission(Permission::WriteAccount)?;
        if actor.account_id == account_id && !enabled {
            return Err(DataError::Conflict("You cannot disable your own login".into()).into());
        }
        let mut credential = self.credential(account_id).await?;
        if credential.enabled == enabled {
            return Ok(());
        }
        let expected = credential.revision;
        credential.enabled = enabled;
        credential.revision = Uuid::new_v4();
        credential.reset_hash = None;
        credential.reset_expires_at = None;
        self.store.local_credential.replace(credential, expected).await?;
        debug!(%account_id, %enabled, "Local login state changed; sessions revoked");
        Ok(())
    }

    async fn reset_local_login(&self, actor: User, account_id: Uuid) -> Result<LocalLoginReset, ApplicationError> {
        actor.check_permission(Permission::WriteAccount)?;
        if actor.account_id == account_id {
            return Err(DataError::Conflict("Use Change password for your own login".into()).into());
        }
        let mut credential = self.credential(account_id).await?;
        if !credential.enabled {
            return Err(DataError::Conflict("Enable local login before resetting its password".into()).into());
        }
        let expected = credential.revision;
        let (grant, hash) = Self::reset_grant();
        credential.password_hash = None;
        credential.revision = Uuid::new_v4();
        credential.reset_hash = Some(hash);
        credential.reset_expires_at = Some(grant.expires_at);
        self.store.local_credential.replace(credential, expected).await?;
        debug!(%account_id, "Local login reset issued; sessions revoked");
        Ok(grant)
    }

    async fn reset_local_password(&self, code: String, new_password: String) -> Result<(), ApplicationError> {
        self.require_local()?;
        if code.len() != 43 {
            return Err(AuthenticationError::InvalidCredentials.into());
        }
        let mut credential = self
            .store
            .local_credential
            .find_by_reset_hash(&Self::reset_hash(&code))
            .await?
            .filter(|c| c.enabled && c.reset_expires_at.is_some_and(|t| t > Utc::now()))
            .ok_or(AuthenticationError::InvalidCredentials)?;
        PasswordService::validate(&new_password)?;
        let expected = credential.revision;
        credential.password_hash = Some(self.passwords.hash(new_password).await?);
        credential.revision = Uuid::new_v4();
        credential.reset_hash = None;
        credential.reset_expires_at = None;
        self.store.local_credential.replace(credential, expected).await?;
        debug!("Local password reset completed");
        Ok(())
    }

    async fn authenticate_jwt(&self, token: &str) -> Result<User, ApplicationError> {
        trace!("Start JWT authentication");

        let claims = self.jwt_authenticator.decode(token).await?;
        let account = match self.store.account.find_by_identity(self.provider, &claims.sub).await? {
            Some(account) => account,
            None if self.provider == AuthProvider::OAuth2 => {
                self.store.account.upsert(self.provider, &claims.sub, None, &[]).await?
            }
            None => return Err(AuthenticationError::InvalidCredentials.into()),
        };
        let permissions = if self.provider == AuthProvider::Jwt {
            let credential = self
                .store
                .local_credential
                .find(account.id)
                .await?
                .ok_or(AuthenticationError::InvalidCredentials)?;
            if !credential.enabled
                || credential.password_hash.is_none()
                || claims.local_identity_id != Some(credential.identity_id)
                || claims.credential_revision != Some(credential.revision)
            {
                return Err(AuthenticationError::InvalidCredentials.into());
            }
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
            account::{Account, ApiKey, AuthClaims, AuthIdentity},
            asset::{Asset, Protocol, NATIVE_ASSET_REF},
            bitcoin::BtcNetwork,
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
            local_identity_id: None,
            credential_revision: None,
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
            wallets: Vec::new(),
            created_at: Utc::now(),
            updated_at: None,
        }
    }

    mod sign_up {
        use super::*;

        mod when_provider_is_oauth2 {
            use super::*;
            #[tokio::test]
            async fn rejects_local_setup() {
                let service = service(
                    MockJWTAuthenticator::new(),
                    MockAppStoreBuilder::new(),
                    AuthProvider::OAuth2,
                );
                assert!(matches!(
                    service.sign_up("a long password phrase".into()).await,
                    Err(ApplicationError::Authentication(
                        AuthenticationError::UnsupportedOperation
                    ))
                ));
            }
        }

        mod when_setup_is_complete {
            use super::*;
            #[tokio::test]
            async fn never_reopens_registration() {
                let mut store = MockAppStoreBuilder::new();
                store
                    .config
                    .expect_find()
                    .withf(|key| key == LOCAL_AUTH_INITIALIZED_KEY)
                    .times(1)
                    .returning(|_| Ok(Some(true.into())));
                let service = service(MockJWTAuthenticator::new(), store, AuthProvider::Jwt);
                assert!(matches!(
                    service.sign_up("a long password phrase".into()).await,
                    Err(ApplicationError::Data(DataError::Conflict(_)))
                ));
            }
        }
    }

    mod create_local_login {
        use super::*;
        mod with_an_invalid_username {
            use super::*;
            #[tokio::test]
            async fn rejects_it_before_creating_credentials() {
                let mut store = MockAppStoreBuilder::new();
                store.account.expect_find().times(1).returning(|id| {
                    Ok(Some(Account {
                        id,
                        ..Default::default()
                    }))
                });
                let service = service(MockJWTAuthenticator::new(), store, AuthProvider::Jwt);
                let result = service
                    .create_local_login(Uuid::new_v4(), "name with spaces".into())
                    .await;
                assert!(matches!(result, Err(ApplicationError::Data(DataError::Validation(_)))));
            }
        }
        mod with_a_valid_username {
            use super::*;
            #[tokio::test]
            async fn stores_only_a_hash_of_the_activation_code() {
                let id = Uuid::new_v4();
                let mut store = MockAppStoreBuilder::new();
                store.account.expect_find().times(1).returning(|id| {
                    Ok(Some(Account {
                        id,
                        ..Default::default()
                    }))
                });
                store
                    .local_credential
                    .expect_create()
                    .withf(move |account_id, subject, hash, expires| {
                        *account_id == id && subject == "alice" && hash.len() == 64 && *expires > Utc::now()
                    })
                    .times(1)
                    .returning(|_, _, _, _| Ok(()));
                let service = service(MockJWTAuthenticator::new(), store, AuthProvider::Jwt);
                let grant = service.create_local_login(id, " Alice ".into()).await.unwrap();
                assert_eq!(grant.code.len(), 43);
            }
        }
    }

    mod activate_legacy_identity {
        use super::*;
        #[tokio::test]
        async fn retains_an_existing_subject_and_account_id() {
            let id = Uuid::new_v4();
            let mut store = MockAppStoreBuilder::new();
            store
                .account
                .expect_find()
                .times(1)
                .returning(move |_| Ok(Some(account_fixture(id, AuthProvider::Jwt, "Existing|Subject", vec![]))));
            store
                .local_credential
                .expect_create()
                .withf(move |account_id, subject, _, _| *account_id == id && subject == "Existing|Subject")
                .times(1)
                .returning(|_, _, _, _| Ok(()));
            let service = service(MockJWTAuthenticator::new(), store, AuthProvider::Jwt);
            assert!(service.create_local_login(id, "Existing|Subject".into()).await.is_ok());
        }
    }

    mod update_local_login {
        use super::*;
        mod when_disabling_ones_own_login {
            use super::*;
            #[tokio::test]
            async fn rejects_self_lockout() {
                let service = service(
                    MockJWTAuthenticator::new(),
                    MockAppStoreBuilder::new(),
                    AuthProvider::Jwt,
                );
                let id = Uuid::new_v4();
                let user = User {
                    account_id: id,
                    permissions: vec![Permission::WriteAccount],
                };
                assert!(matches!(
                    service.update_local_login(user, id, false).await,
                    Err(ApplicationError::Data(DataError::Conflict(_)))
                ));
            }
        }
        mod without_account_administration {
            use super::*;
            #[tokio::test]
            async fn rejects_the_operation() {
                let service = service(
                    MockJWTAuthenticator::new(),
                    MockAppStoreBuilder::new(),
                    AuthProvider::Jwt,
                );
                assert!(matches!(
                    service.update_local_login(User::default(), Uuid::new_v4(), false).await,
                    Err(ApplicationError::Authorization(_))
                ));
            }
        }
    }

    mod authenticate_jwt {
        use super::*;

        mod when_oauth2_token_is_valid {
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
                    .withf(|provider, subject| *provider == AuthProvider::OAuth2 && subject == "alice")
                    .times(1)
                    .returning(|_, _| Ok(None));
                store
                    .account
                    .expect_upsert()
                    .withf(|provider, subject, display_name, granted| {
                        *provider == AuthProvider::OAuth2
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

                let service = service(jwt, store, AuthProvider::OAuth2);

                let user = service.authenticate_jwt("token").await.unwrap();

                assert_eq!(user.account_id, account_id);
                assert_eq!(user.permissions, vec![Permission::ReadWallet]);
            }
        }

        mod when_a_local_identity_was_deleted {
            use super::*;
            #[tokio::test]
            async fn rejects_the_token_without_recreating_the_account() {
                let mut jwt = MockJWTAuthenticator::new();
                jwt.expect_decode().times(1).returning(|_| Ok(claims("alice")));
                let mut store = MockAppStoreBuilder::new();
                store
                    .account
                    .expect_find_by_identity()
                    .times(1)
                    .returning(|_, _| Ok(None));
                let service = service(jwt, store, AuthProvider::Jwt);
                assert!(matches!(
                    service.authenticate_jwt("old-token").await,
                    Err(ApplicationError::Authentication(
                        AuthenticationError::InvalidCredentials
                    ))
                ));
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

    mod reset_local_password {
        use super::*;
        mod when_the_code_has_expired {
            use super::*;
            #[tokio::test]
            async fn rejects_it_without_changing_the_credential() {
                let mut store = MockAppStoreBuilder::new();
                store
                    .local_credential
                    .expect_find_by_reset_hash()
                    .times(1)
                    .returning(|_| {
                        Ok(Some(LocalCredential {
                            account_id: Uuid::new_v4(),
                            identity_id: Uuid::new_v4(),
                            subject: "alice".into(),
                            enabled: true,
                            password_hash: None,
                            revision: Uuid::new_v4(),
                            reset_hash: Some("hash".into()),
                            reset_expires_at: Some(Utc::now() - Duration::seconds(1)),
                        }))
                    });
                let service = service(MockJWTAuthenticator::new(), store, AuthProvider::Jwt);
                assert!(matches!(
                    service
                        .reset_local_password("c".repeat(43), "a long password phrase".into())
                        .await,
                    Err(ApplicationError::Authentication(
                        AuthenticationError::InvalidCredentials
                    ))
                ));
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
