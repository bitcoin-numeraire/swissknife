use async_trait::async_trait;
use swissknife_types::{AccountNotification, NotificationAction};
use uuid::Uuid;

use crate::application::errors::DatabaseError;

#[cfg_attr(test, mockall::automock)]
#[async_trait]
pub trait NotificationRepository: Send + Sync {
    async fn list(
        &self,
        account_id: Uuid,
        before: Option<i32>,
        limit: u64,
        archived: bool,
    ) -> Result<Vec<AccountNotification>, DatabaseError>;
    async fn unread_count(&self, account_id: Uuid) -> Result<u64, DatabaseError>;
    async fn update(&self, account_id: Uuid, id: i32, action: NotificationAction) -> Result<bool, DatabaseError>;
    async fn read_through(&self, account_id: Uuid, through: i32) -> Result<(), DatabaseError>;
}
