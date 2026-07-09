use async_trait::async_trait;
use base64::{prelude::BASE64_STANDARD, Engine};
use chrono::{Duration, Utc};
use serde_bolt::bitcoin::hashes::{sha256, Hash};
use tracing::{debug, info, trace};
use uuid::Uuid;

use swissknife_types::CreateApiKeyRequest;

use crate::application::{
    composition::AppStore,
    errors::{ApplicationError, DataError},
};

use super::{ApiKey, ApiKeyFilter, ApiKeyUseCases, User};

const MAX_ALLOWED_EXPIRY_SECONDS: u32 = 31_536_000; // 1 year in seconds

pub struct ApiKeyService {
    store: AppStore,
}

impl ApiKeyService {
    pub fn new(store: AppStore) -> Self {
        ApiKeyService { store }
    }
}

#[async_trait]
impl ApiKeyUseCases for ApiKeyService {
    async fn generate(&self, user: User, request: CreateApiKeyRequest) -> Result<ApiKey, ApplicationError> {
        debug!(account_id = ?request.account_id, "Generating API key");

        // Validate that requested permissions are a subset of user's permissions
        if !request.permissions.iter().all(|p| user.has_permission(p.clone())) {
            return Err(DataError::Validation("Invalid permissions".to_string()).into());
        }

        let expires_at = match request.expiry {
            Some(seconds) => {
                if seconds > MAX_ALLOWED_EXPIRY_SECONDS {
                    return Err(DataError::Validation("Expiry too far in the future".to_string()).into());
                }

                Some(Utc::now() + Duration::seconds(seconds as i64))
            }
            None => None,
        };

        let account_id = request.account_id.expect("account_id should be defined");

        // Generate a new API key
        let bytes: [u8; 32] = rand::random();
        let api_key_plain = BASE64_STANDARD.encode(bytes);
        let key_hash = sha256::Hash::hash(&bytes).to_byte_array().to_vec();

        let api_key = ApiKey {
            account_id,
            name: request.name,
            key_hash,
            permissions: request.permissions.clone(),
            expires_at,
            description: request.description,
            ..Default::default()
        };

        let mut api_key = self.store.api_key.insert(api_key).await?;
        api_key.key = Some(api_key_plain);

        info!(id = %api_key.id, "API key generated successfully");
        Ok(api_key)
    }

    async fn get(&self, id: Uuid) -> Result<ApiKey, ApplicationError> {
        trace!(%id, "Fetching API key");

        let api_key = self
            .store
            .api_key
            .find(id)
            .await?
            .ok_or_else(|| DataError::NotFound("API key not found.".to_string()))?;

        debug!(%id, "API key fetched successfully");
        Ok(api_key)
    }

    async fn list(&self, filter: ApiKeyFilter) -> Result<Vec<ApiKey>, ApplicationError> {
        trace!(?filter, "Listing API keys");

        let api_keys = self.store.api_key.find_many(filter.clone()).await?;

        debug!(?filter, "API keys listed successfully");
        Ok(api_keys)
    }

    async fn revoke(&self, id: Uuid) -> Result<(), ApplicationError> {
        debug!(%id, "Revoking API key");

        let n_deleted = self
            .store
            .api_key
            .delete_many(ApiKeyFilter {
                ids: Some(vec![id]),
                ..Default::default()
            })
            .await?;

        if n_deleted == 0 {
            return Err(DataError::NotFound("API key not found.".to_string()).into());
        }

        info!(%id, "API key revoked successfully");
        Ok(())
    }

    async fn revoke_many(&self, filter: ApiKeyFilter) -> Result<u64, ApplicationError> {
        debug!(?filter, "Revoking API keys");

        let n_deleted = self.store.api_key.delete_many(filter.clone()).await?;

        info!(?filter, n_deleted, "API keys revoked successfully");
        Ok(n_deleted)
    }
}

#[cfg(test)]
mod tests {
    use crate::{
        application::{composition::MockAppStoreBuilder, errors::DatabaseError},
        domains::user::Permission,
    };

    use super::*;

    fn user_with(permissions: Vec<Permission>) -> User {
        User {
            account_id: Uuid::new_v4(),
            permissions,
        }
    }

    fn create_request(permissions: Vec<Permission>, expiry: Option<u32>) -> CreateApiKeyRequest {
        CreateApiKeyRequest {
            account_id: None,
            name: "primary".to_string(),
            permissions,
            description: None,
            expiry,
        }
    }

    mod generate {
        use super::*;

        mod with_permitted_permissions_and_no_expiry {
            use super::*;

            #[tokio::test]
            async fn inserts_key_and_returns_plaintext_secret() {
                let user = user_with(vec![Permission::ReadWallet, Permission::WriteWallet]);
                let account_id = user.account_id;
                let mut request = create_request(vec![Permission::ReadWallet], None);
                request.account_id = Some(account_id);
                let mut store = MockAppStoreBuilder::new();
                store
                    .api_key
                    .expect_insert()
                    .withf(move |api_key| {
                        api_key.account_id == account_id
                            && api_key.permissions == vec![Permission::ReadWallet]
                            && api_key.expires_at.is_none()
                            && !api_key.key_hash.is_empty()
                    })
                    .times(1)
                    .returning(Ok);

                let service = ApiKeyService::new(store.build());

                let api_key = service.generate(user, request).await.unwrap();

                // The plaintext secret is only attached on creation.
                assert!(api_key.key.is_some());
            }
        }

        mod for_an_explicit_account {
            use super::*;

            #[tokio::test]
            async fn attaches_the_key_to_the_requested_account() {
                let user = user_with(vec![Permission::ReadWallet, Permission::WriteWallet]);
                let target_account_id = Uuid::new_v4();
                let mut request = create_request(vec![Permission::ReadWallet], None);
                request.account_id = Some(target_account_id);

                let mut store = MockAppStoreBuilder::new();
                store
                    .api_key
                    .expect_insert()
                    .withf(move |api_key| {
                        api_key.account_id == target_account_id && api_key.permissions == vec![Permission::ReadWallet]
                    })
                    .times(1)
                    .returning(Ok);

                let service = ApiKeyService::new(store.build());

                let api_key = service.generate(user, request).await.unwrap();

                assert!(api_key.key.is_some());
            }
        }

        mod with_expiry_within_the_limit {
            use super::*;

            #[tokio::test]
            async fn sets_an_expiration() {
                let user = user_with(vec![Permission::ReadWallet]);
                let account_id = user.account_id;
                let mut request = create_request(vec![Permission::ReadWallet], Some(3_600));
                request.account_id = Some(account_id);
                let mut store = MockAppStoreBuilder::new();
                store
                    .api_key
                    .expect_insert()
                    .withf(|api_key| api_key.expires_at.is_some())
                    .times(1)
                    .returning(Ok);

                let service = ApiKeyService::new(store.build());

                let api_key = service.generate(user, request).await.unwrap();

                assert!(api_key.expires_at.is_some());
            }
        }

        mod with_permissions_beyond_the_user {
            use super::*;

            #[tokio::test]
            async fn rejects_with_validation_error() {
                // No insert expected: the request must be rejected before persistence.
                let service = ApiKeyService::new(MockAppStoreBuilder::new().build());

                let err = service
                    .generate(
                        user_with(vec![Permission::ReadWallet]),
                        create_request(vec![Permission::WriteWallet], None),
                    )
                    .await
                    .unwrap_err();

                assert!(matches!(err, ApplicationError::Data(DataError::Validation(_))));
            }
        }

        mod with_expiry_beyond_the_limit {
            use super::*;

            #[tokio::test]
            async fn rejects_with_validation_error() {
                let service = ApiKeyService::new(MockAppStoreBuilder::new().build());

                let err = service
                    .generate(
                        user_with(vec![Permission::ReadWallet]),
                        create_request(vec![Permission::ReadWallet], Some(MAX_ALLOWED_EXPIRY_SECONDS + 1)),
                    )
                    .await
                    .unwrap_err();

                assert!(matches!(err, ApplicationError::Data(DataError::Validation(_))));
            }
        }

        mod when_insert_fails {
            use super::*;

            #[tokio::test]
            async fn propagates_database_error() {
                let user = user_with(vec![Permission::ReadWallet]);
                let account_id = user.account_id;
                let mut request = create_request(vec![Permission::ReadWallet], None);
                request.account_id = Some(account_id);
                let mut store = MockAppStoreBuilder::new();
                store
                    .api_key
                    .expect_insert()
                    .times(1)
                    .returning(|_| Err(DatabaseError::Insert("boom".to_string())));

                let service = ApiKeyService::new(store.build());

                let err = service.generate(user, request).await.unwrap_err();

                assert!(matches!(err, ApplicationError::Database(DatabaseError::Insert(_))));
            }
        }
    }

    mod get {
        use super::*;

        mod when_found {
            use super::*;

            #[tokio::test]
            async fn returns_api_key() {
                let id = Uuid::new_v4();

                let mut store = MockAppStoreBuilder::new();
                store.api_key.expect_find().times(1).returning(move |id| {
                    Ok(Some(ApiKey {
                        id,
                        ..Default::default()
                    }))
                });

                let service = ApiKeyService::new(store.build());

                assert_eq!(service.get(id).await.unwrap().id, id);
            }
        }

        mod when_missing {
            use super::*;

            #[tokio::test]
            async fn returns_not_found() {
                let mut store = MockAppStoreBuilder::new();
                store.api_key.expect_find().times(1).returning(|_| Ok(None));

                let service = ApiKeyService::new(store.build());

                let err = service.get(Uuid::new_v4()).await.unwrap_err();

                assert!(matches!(err, ApplicationError::Data(DataError::NotFound(_))));
            }
        }
    }

    mod revoke {
        use super::*;

        mod when_a_key_is_removed {
            use super::*;

            #[tokio::test]
            async fn succeeds() {
                let mut store = MockAppStoreBuilder::new();
                store.api_key.expect_delete_many().times(1).returning(|_| Ok(1));

                let service = ApiKeyService::new(store.build());

                assert!(service.revoke(Uuid::new_v4()).await.is_ok());
            }
        }

        mod when_nothing_is_removed {
            use super::*;

            #[tokio::test]
            async fn returns_not_found() {
                let mut store = MockAppStoreBuilder::new();
                store.api_key.expect_delete_many().times(1).returning(|_| Ok(0));

                let service = ApiKeyService::new(store.build());

                let err = service.revoke(Uuid::new_v4()).await.unwrap_err();

                assert!(matches!(err, ApplicationError::Data(DataError::NotFound(_))));
            }
        }
    }

    mod revoke_many {
        use super::*;

        #[tokio::test]
        async fn returns_revoked_count() {
            let mut store = MockAppStoreBuilder::new();
            store.api_key.expect_delete_many().times(1).returning(|_| Ok(4));

            let service = ApiKeyService::new(store.build());

            assert_eq!(service.revoke_many(ApiKeyFilter::default()).await.unwrap(), 4);
        }
    }
}
