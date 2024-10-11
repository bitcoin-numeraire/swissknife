use async_trait::async_trait;
use base64::{prelude::BASE64_STANDARD, Engine};
use chrono::{Duration, Utc};
use rand::rngs::OsRng;
use rand::RngCore;
use serde_bolt::bitcoin::hashes::{sha256, Hash};
use tracing::{debug, info, trace};
use uuid::Uuid;

use crate::application::{
    dtos::CreateApiKeyRequest,
    entities::AppStore,
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
    async fn generate(
        &self,
        user: User,
        request: CreateApiKeyRequest,
    ) -> Result<ApiKey, ApplicationError> {
        debug!(user_id = request.user_id, "Generating API key");

        // Validate that requested permissions are a subset of user's permissions
        if !request
            .permissions
            .iter()
            .all(|p| user.has_permission(p.clone()))
        {
            return Err(DataError::Validation("Invalid permissions".to_string()).into());
        }

        let expires_at = match request.expiry {
            Some(seconds) => {
                if seconds > MAX_ALLOWED_EXPIRY_SECONDS {
                    return Err(
                        DataError::Validation("Expiry too far in the future".to_string()).into(),
                    );
                }

                Some(Utc::now() + Duration::seconds(seconds as i64))
            }
            None => None,
        };

        // Generate a new API key
        let mut bytes = [0u8; 32];
        OsRng.fill_bytes(&mut bytes);
        let api_key_plain = BASE64_STANDARD.encode(&bytes);
        let key_hash = sha256::Hash::hash(&bytes).to_vec();

        let api_key = ApiKey {
            user_id: request.user_id.expect("user_id should be defined"),
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
