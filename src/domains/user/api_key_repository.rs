use async_trait::async_trait;
use uuid::Uuid;

use crate::application::errors::DatabaseError;

use super::{ApiKey, ApiKeyFilter};

#[cfg_attr(test, mockall::automock)]
#[async_trait]
pub trait ApiKeyRepository: Send + Sync {
    async fn find(&self, id: Uuid) -> Result<Option<ApiKey>, DatabaseError>;
    async fn find_by_key_hash(&self, key_hash: Vec<u8>) -> Result<Option<ApiKey>, DatabaseError>;
    async fn find_many(&self, filter: ApiKeyFilter) -> Result<Vec<ApiKey>, DatabaseError>;
    async fn insert(&self, api_key: ApiKey) -> Result<ApiKey, DatabaseError>;
    async fn delete_many(&self, filter: ApiKeyFilter) -> Result<u64, DatabaseError>;
}
