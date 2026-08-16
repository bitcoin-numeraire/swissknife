use async_trait::async_trait;
use chrono::{DateTime, Utc};
use uuid::Uuid;

use crate::application::errors::DatabaseError;

use super::ClientEvent;

#[cfg_attr(test, mockall::automock)]
#[async_trait]
pub trait ClientEventRepository: Send + Sync {
    async fn latest_id(&self, account_id: Uuid) -> Result<Option<i32>, DatabaseError>;
    async fn find_after(&self, account_id: Uuid, after_id: i32, limit: u64) -> Result<Vec<ClientEvent>, DatabaseError>;
    async fn pruned_through(&self) -> Result<i32, DatabaseError>;
    async fn prune_before(&self, cutoff: DateTime<Utc>) -> Result<u64, DatabaseError>;
}
