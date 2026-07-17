use chrono::{DateTime, Utc};
use uuid::Uuid;

pub use swissknife_types::{
    CreateWebhookSubscriptionRequest, CreatedWebhookSubscription, RotateWebhookSecretResponse,
    UpdateWebhookSubscriptionRequest, WebhookDelivery, WebhookDeliveryStatus, WebhookSubscription,
};

use super::{ClientEvent, ClientEventType};

#[derive(Clone, Debug)]
pub struct NewWebhookSubscription {
    pub id: Uuid,
    pub account_id: Uuid,
    pub wallet_id: Uuid,
    pub url: String,
    pub event_types: Vec<ClientEventType>,
    pub signing_secret: String,
    pub last_event_id: i32,
}

#[derive(Clone, Debug)]
pub struct StoredWebhookSubscription {
    pub id: Uuid,
    pub account_id: Uuid,
    pub wallet_id: Uuid,
    pub url: String,
    pub event_types: Vec<ClientEventType>,
    pub signing_secret: String,
    pub active: bool,
    pub last_event_id: i32,
    pub created_at: DateTime<Utc>,
    pub updated_at: Option<DateTime<Utc>>,
}

impl From<StoredWebhookSubscription> for WebhookSubscription {
    fn from(value: StoredWebhookSubscription) -> Self {
        Self {
            id: value.id,
            account_id: value.account_id,
            wallet_id: value.wallet_id,
            url: value.url,
            event_types: value.event_types,
            active: value.active,
            created_at: value.created_at,
            updated_at: value.updated_at,
        }
    }
}

#[derive(Clone, Debug)]
pub struct ClaimedWebhookDelivery {
    pub id: Uuid,
    pub subscription_id: Uuid,
    pub event: ClientEvent,
    pub url: String,
    pub signing_secret: String,
    pub attempt_count: u32,
}
