use async_trait::async_trait;
use uuid::Uuid;

use crate::application::errors::ApplicationError;

use super::ClientEvent;

#[cfg_attr(test, mockall::automock)]
#[async_trait]
pub trait ClientEventUseCases: Send + Sync {
    async fn latest_id(&self, wallet_id: Uuid) -> Result<i32, ApplicationError>;
    async fn list_after(&self, wallet_id: Uuid, after_id: i32) -> Result<Vec<ClientEvent>, ApplicationError>;
}
