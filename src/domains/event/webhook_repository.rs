use async_trait::async_trait;
use chrono::{DateTime, Utc};
use uuid::Uuid;

use crate::application::errors::DatabaseError;

use super::{ClaimedWebhookDelivery, NewWebhookSubscription, StoredWebhookSubscription, WebhookDelivery};

#[cfg_attr(test, mockall::automock)]
#[async_trait]
pub trait WebhookRepository: Send + Sync {
    async fn insert(&self, subscription: NewWebhookSubscription) -> Result<StoredWebhookSubscription, DatabaseError>;
    async fn find_many(
        &self,
        account_id: Uuid,
        wallet_id: Uuid,
    ) -> Result<Vec<StoredWebhookSubscription>, DatabaseError>;
    async fn find_owned(
        &self,
        account_id: Uuid,
        wallet_id: Uuid,
        id: Uuid,
    ) -> Result<Option<StoredWebhookSubscription>, DatabaseError>;
    async fn update(&self, subscription: StoredWebhookSubscription)
        -> Result<StoredWebhookSubscription, DatabaseError>;
    async fn cancel_pending(&self, subscription_id: Uuid, reason: String) -> Result<u64, DatabaseError>;
    async fn delete_owned(&self, account_id: Uuid, wallet_id: Uuid, id: Uuid) -> Result<u64, DatabaseError>;
    async fn list_deliveries(
        &self,
        account_id: Uuid,
        wallet_id: Uuid,
        subscription_id: Uuid,
        limit: u64,
    ) -> Result<Vec<WebhookDelivery>, DatabaseError>;

    /// Fan matching durable events into delivery rows and atomically advance each subscription cursor.
    async fn prepare_deliveries(&self, batch_size: u64) -> Result<u64, DatabaseError>;
    /// Claim due delivery rows with a lease so multiple workers do not send the same attempt concurrently.
    async fn claim_due(
        &self,
        now: DateTime<Utc>,
        locked_until: DateTime<Utc>,
        limit: u64,
    ) -> Result<Vec<ClaimedWebhookDelivery>, DatabaseError>;
    async fn mark_delivered(&self, id: Uuid, response_status: u16) -> Result<(), DatabaseError>;
    async fn mark_failed(
        &self,
        id: Uuid,
        response_status: Option<u16>,
        error: String,
        next_attempt_at: DateTime<Utc>,
        exhausted: bool,
    ) -> Result<(), DatabaseError>;
}
