use std::sync::Arc;

use async_trait::async_trait;
use base64::{engine::general_purpose::URL_SAFE_NO_PAD, Engine};
use chrono::{Duration, Utc};
use serde_bolt::bitcoin::hashes::{sha256, Hash};
use swissknife_types::LocalLoginReset;
use tracing::debug;
use uuid::Uuid;

use crate::application::{
    composition::AuthProvider,
    errors::{ApplicationError, AuthenticationError, DataError},
};

use super::{LocalCredentialRepository, LoginRecoveryUseCases};

/// Operator recovery is exposed only through the CLI, never an HTTP route.
pub struct LoginRecoveryService {
    repository: Arc<dyn LocalCredentialRepository>,
    provider: AuthProvider,
}

impl LoginRecoveryService {
    pub fn new(repository: Arc<dyn LocalCredentialRepository>, provider: AuthProvider) -> Self {
        Self { repository, provider }
    }
}

#[async_trait]
impl LoginRecoveryUseCases for LoginRecoveryService {
    async fn recover(&self, account_id: Uuid) -> Result<LocalLoginReset, ApplicationError> {
        if self.provider != AuthProvider::Jwt {
            return Err(AuthenticationError::UnsupportedOperation.into());
        }
        let mut credential = self
            .repository
            .find(account_id)
            .await?
            .ok_or_else(|| DataError::NotFound("Existing local login not found".into()))?;
        let expected = credential.revision;
        let bytes: [u8; 32] = rand::random();
        let code = URL_SAFE_NO_PAD.encode(bytes);
        let expires_at = Utc::now() + Duration::minutes(30);
        credential.enabled = true;
        credential.password_hash = None;
        credential.reset_hash = Some(sha256::Hash::hash(code.as_bytes()).to_string());
        credential.reset_expires_at = Some(expires_at);
        credential.revision = Uuid::new_v4();
        self.repository.replace(credential, expected).await?;
        debug!(%account_id, "Operator recovered local login; sessions revoked");
        Ok(LocalLoginReset { code, expires_at })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::domains::account::{LocalCredential, MockLocalCredentialRepository};

    fn credential(account_id: Uuid, revision: Uuid) -> LocalCredential {
        LocalCredential {
            account_id,
            identity_id: Uuid::new_v4(),
            subject: "alice".into(),
            password_hash: Some("old-hash".into()),
            enabled: false,
            revision,
            reset_hash: Some("old-reset".into()),
            reset_expires_at: Some(Utc::now()),
        }
    }

    mod recover {
        use super::*;

        mod with_an_existing_login {
            use super::*;
            #[tokio::test]
            async fn enables_login_and_revokes_the_password_and_existing_sessions() {
                let id = Uuid::new_v4();
                let revision = Uuid::new_v4();
                let original = credential(id, revision);
                let identity_id = original.identity_id;
                let saved = Arc::new(std::sync::Mutex::new(None));
                let captured = saved.clone();
                let mut repository = MockLocalCredentialRepository::new();
                repository
                    .expect_find()
                    .with(mockall::predicate::eq(id))
                    .times(1)
                    .return_once(move |_| Ok(Some(original)));
                repository
                    .expect_replace()
                    .withf(move |value, expected| {
                        value.account_id == id
                            && value.identity_id == identity_id
                            && value.subject == "alice"
                            && *expected == revision
                            && value.revision != revision
                            && value.enabled
                            && value.password_hash.is_none()
                    })
                    .times(1)
                    .return_once(move |value, _| {
                        *captured.lock().unwrap() = Some(value);
                        Ok(())
                    });
                let service = LoginRecoveryService::new(Arc::new(repository), AuthProvider::Jwt);
                let before = Utc::now();
                let grant = service.recover(id).await.unwrap();
                assert_eq!(URL_SAFE_NO_PAD.decode(&grant.code).unwrap().len(), 32);
                assert!(grant.expires_at >= before + Duration::minutes(30));
                assert!(grant.expires_at <= Utc::now() + Duration::minutes(30));
                let stored = saved.lock().unwrap().take().unwrap();
                assert_eq!(
                    stored.reset_hash,
                    Some(sha256::Hash::hash(grant.code.as_bytes()).to_string())
                );
                assert_eq!(stored.reset_expires_at, Some(grant.expires_at));
            }
        }

        mod when_the_provider_is_oauth2 {
            use super::*;
            #[tokio::test]
            async fn rejects_recovery_without_accessing_credentials() {
                let service =
                    LoginRecoveryService::new(Arc::new(MockLocalCredentialRepository::new()), AuthProvider::OAuth2);
                assert!(matches!(
                    service.recover(Uuid::new_v4()).await,
                    Err(ApplicationError::Authentication(
                        AuthenticationError::UnsupportedOperation
                    ))
                ));
            }
        }

        mod when_the_login_does_not_exist {
            use super::*;
            #[tokio::test]
            async fn returns_not_found_without_creating_credentials() {
                let id = Uuid::new_v4();
                let mut repository = MockLocalCredentialRepository::new();
                repository
                    .expect_find()
                    .with(mockall::predicate::eq(id))
                    .times(1)
                    .returning(|_| Ok(None));
                let service = LoginRecoveryService::new(Arc::new(repository), AuthProvider::Jwt);
                assert!(matches!(
                    service.recover(id).await,
                    Err(ApplicationError::Data(DataError::NotFound(_)))
                ));
            }
        }

        mod when_reading_credentials_fails {
            use super::*;
            #[tokio::test]
            async fn propagates_the_error_without_writing() {
                let id = Uuid::new_v4();
                let mut repository = MockLocalCredentialRepository::new();
                repository
                    .expect_find()
                    .with(mockall::predicate::eq(id))
                    .times(1)
                    .returning(|_| Err(DataError::Inconsistency("read failed".into()).into()));
                let service = LoginRecoveryService::new(Arc::new(repository), AuthProvider::Jwt);
                assert!(matches!(
                    service.recover(id).await,
                    Err(ApplicationError::Data(DataError::Inconsistency(_)))
                ));
            }
        }

        mod when_credentials_change_concurrently {
            use super::*;
            #[tokio::test]
            async fn propagates_the_conflict_instead_of_returning_a_reset_code() {
                let id = Uuid::new_v4();
                let revision = Uuid::new_v4();
                let mut repository = MockLocalCredentialRepository::new();
                repository
                    .expect_find()
                    .with(mockall::predicate::eq(id))
                    .times(1)
                    .returning(move |_| Ok(Some(credential(id, revision))));
                repository
                    .expect_replace()
                    .withf(move |value, expected| value.account_id == id && *expected == revision)
                    .times(1)
                    .returning(|_, _| Err(DataError::Conflict("changed".into()).into()));
                let service = LoginRecoveryService::new(Arc::new(repository), AuthProvider::Jwt);
                assert!(matches!(
                    service.recover(id).await,
                    Err(ApplicationError::Data(DataError::Conflict(_)))
                ));
            }
        }
    }
}
