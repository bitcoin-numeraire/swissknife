use async_trait::async_trait;
use uuid::Uuid;

use crate::application::errors::ApplicationError;

use super::{
    CreateWebhookSubscriptionRequest, CreatedWebhookSubscription, RotateWebhookSecretResponse,
    UpdateWebhookSubscriptionRequest, WebhookDelivery, WebhookSubscription, WebhookSubscriptionFilter,
};

#[cfg_attr(test, mockall::automock)]
#[async_trait]
pub trait WebhookUseCases: Send + Sync {
    async fn create(
        &self,
        wallet_id: Uuid,
        request: CreateWebhookSubscriptionRequest,
    ) -> Result<CreatedWebhookSubscription, ApplicationError>;
    async fn get(&self, id: Uuid) -> Result<WebhookSubscription, ApplicationError>;
    async fn get_by_account_id(
        &self,
        account_id: Uuid,
        wallet_id: Uuid,
        id: Uuid,
    ) -> Result<WebhookSubscription, ApplicationError>;
    async fn list(&self, filter: WebhookSubscriptionFilter) -> Result<Vec<WebhookSubscription>, ApplicationError>;
    async fn update(
        &self,
        id: Uuid,
        request: UpdateWebhookSubscriptionRequest,
    ) -> Result<WebhookSubscription, ApplicationError>;
    async fn delete(&self, id: Uuid) -> Result<(), ApplicationError>;
    async fn rotate_secret(&self, id: Uuid) -> Result<RotateWebhookSecretResponse, ApplicationError>;
    async fn list_deliveries(&self, subscription_id: Uuid) -> Result<Vec<WebhookDelivery>, ApplicationError>;
}
