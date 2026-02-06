use async_trait::async_trait;
use serde_json::Value;

use crate::application::errors::DatabaseError;

#[async_trait]
pub trait ConfigRepository: Send + Sync {
    async fn find(&self, key: &str) -> Result<Option<Value>, DatabaseError>;
    async fn insert(&self, key: &str, value: Value) -> Result<(), DatabaseError>;
    async fn upsert(&self, key: &str, value: Value) -> Result<(), DatabaseError>;
}
