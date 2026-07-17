use async_trait::async_trait;
use uuid::Uuid;

use crate::application::errors::DatabaseError;

use super::{ClientEvent, NewClientEvent};

#[cfg_attr(test, mockall::automock)]
#[async_trait]
pub trait ClientEventRepository: Send + Sync {
    async fn append(&self, event: NewClientEvent) -> Result<(), DatabaseError>;
    async fn latest_id(&self, wallet_id: Uuid) -> Result<Option<i32>, DatabaseError>;
    async fn find_after(&self, wallet_id: Uuid, after_id: i32, limit: u64) -> Result<Vec<ClientEvent>, DatabaseError>;
}
