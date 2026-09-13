use async_trait::async_trait;
use uuid::Uuid;

use crate::application::errors::ApplicationError;

use super::ClientEvent;

#[cfg_attr(test, mockall::automock)]
#[async_trait]
pub trait ClientEventUseCases: Send + Sync {
    async fn latest_id(&self, account_id: Uuid) -> Result<i32, ApplicationError>;
    async fn ensure_cursor_available(&self, after_id: i32) -> Result<(), ApplicationError>;
    async fn list_after(&self, account_id: Uuid, after_id: i32) -> Result<Vec<ClientEvent>, ApplicationError>;
    async fn prune(&self) -> Result<u64, ApplicationError>;
}
