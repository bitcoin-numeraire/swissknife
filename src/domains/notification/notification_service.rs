use async_trait::async_trait;
use std::sync::Arc;
use swissknife_types::{NotificationAction, NotificationPage, NotificationQuery};
use uuid::Uuid;

use super::NotificationRepository;
use crate::application::errors::{ApplicationError, DataError};

#[cfg_attr(test, mockall::automock)]
#[async_trait]
pub trait NotificationUseCases: Send + Sync {
    async fn page(&self, account_id: Uuid, query: NotificationQuery) -> Result<NotificationPage, ApplicationError>;
    async fn update(&self, account_id: Uuid, id: i32, action: NotificationAction) -> Result<(), ApplicationError>;
    async fn read_through(&self, account_id: Uuid, through: i32) -> Result<(), ApplicationError>;
}

pub struct NotificationService {
    repo: Arc<dyn NotificationRepository>,
}

impl NotificationService {
    pub fn new(repo: Arc<dyn NotificationRepository>) -> Self {
        Self { repo }
    }
}

#[async_trait]
impl NotificationUseCases for NotificationService {
    async fn page(&self, account_id: Uuid, query: NotificationQuery) -> Result<NotificationPage, ApplicationError> {
        let limit = query.limit.unwrap_or(25);
        if !(1..=100).contains(&limit) || query.before.is_some_and(|id| id <= 0) {
            return Err(DataError::Validation(
                "Use a positive notification cursor and a limit between 1 and 100.".into(),
            )
            .into());
        }
        let (mut items, unread_count) = tokio::try_join!(
            self.repo
                .list(account_id, query.before, u64::from(limit) + 1, query.archived),
            self.repo.unread_count(account_id)
        )?;
        let next_before = if items.len() > limit as usize {
            items.truncate(limit as usize);
            items.last().map(|n| n.source_event_id)
        } else {
            None
        };
        Ok(NotificationPage {
            items,
            unread_count,
            next_before,
        })
    }

    async fn update(&self, account_id: Uuid, id: i32, action: NotificationAction) -> Result<(), ApplicationError> {
        if !self.repo.update(account_id, id, action).await? {
            return Err(DataError::NotFound("Notification".into()).into());
        }
        Ok(())
    }

    async fn read_through(&self, account_id: Uuid, through: i32) -> Result<(), ApplicationError> {
        if through <= 0 {
            return Err(DataError::Validation("The notification cursor must be positive.".into()).into());
        }
        Ok(self.repo.read_through(account_id, through).await?)
    }
}
