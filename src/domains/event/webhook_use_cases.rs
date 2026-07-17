use async_trait::async_trait;
use uuid::Uuid;

use crate::application::errors::ApplicationError;

use super::{
    CreateWebhookSubscriptionRequest, CreatedWebhookSubscription, RotateWebhookSecretResponse,
    UpdateWebhookSubscriptionRequest, WebhookDelivery, WebhookSubscription,
};

#[cfg_attr(test, mockall::automock)]
#[async_trait]
pub trait WebhookUseCases: Send + Sync {
    async fn create(
        &self,
        account_id: Uuid,
        wallet_id: Uuid,
        request: CreateWebhookSubscriptionRequest,
    ) -> Result<CreatedWebhookSubscription, ApplicationError>;
    async fn list(&self, account_id: Uuid, wallet_id: Uuid) -> Result<Vec<WebhookSubscription>, ApplicationError>;
    async fn update(
        &self,
        account_id: Uuid,
        wallet_id: Uuid,
        id: Uuid,
        request: UpdateWebhookSubscriptionRequest,
    ) -> Result<WebhookSubscription, ApplicationError>;
    async fn delete(&self, account_id: Uuid, wallet_id: Uuid, id: Uuid) -> Result<(), ApplicationError>;
    async fn rotate_secret(
        &self,
        account_id: Uuid,
        wallet_id: Uuid,
        id: Uuid,
    ) -> Result<RotateWebhookSecretResponse, ApplicationError>;
    async fn list_deliveries(
        &self,
        account_id: Uuid,
        wallet_id: Uuid,
        subscription_id: Uuid,
    ) -> Result<Vec<WebhookDelivery>, ApplicationError>;
}
