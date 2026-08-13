use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use strum_macros::{Display, EnumString};
use utoipa::ToSchema;
use uuid::Uuid;

/// A durable event emitted after an invoice or payment changes state.
#[derive(Clone, Debug, Deserialize, Serialize, ToSchema)]
pub struct ClientEvent {
    /// Monotonic cursor used by SSE `Last-Event-ID` replay.
    pub id: String,
    /// Stable event name. The SSE `event` field contains the same value.
    pub event_type: ClientEventType,
    /// Wallet whose state changed.
    pub wallet_id: Uuid,
    /// Invoice or payment ID represented by `data`.
    pub resource_id: Uuid,
    /// Full invoice or payment snapshot at the time the event was committed.
    #[schema(value_type = Object)]
    pub data: serde_json::Value,
    /// Time the event was committed to the event log.
    pub created_at: DateTime<Utc>,
}

/// Stable event names shared by SSE and webhook delivery.
#[derive(Clone, Copy, Debug, Deserialize, Display, EnumString, Eq, PartialEq, Serialize, ToSchema)]
pub enum ClientEventType {
    #[serde(rename = "invoice.paid")]
    #[strum(serialize = "invoice.paid")]
    InvoicePaid,
    #[serde(rename = "payment.settled")]
    #[strum(serialize = "payment.settled")]
    PaymentSettled,
    #[serde(rename = "payment.failed")]
    #[strum(serialize = "payment.failed")]
    PaymentFailed,
}

/// Create a server-to-server webhook for one account-owned wallet.
#[derive(Clone, Debug, Deserialize, Serialize, ToSchema)]
pub struct CreateWebhookSubscriptionRequest {
    /// Public HTTPS endpoint that receives signed POST requests.
    pub url: String,
    /// Non-empty event filter.
    pub event_types: Vec<ClientEventType>,
}

/// Update a webhook endpoint, event filter, or enabled state.
#[derive(Clone, Debug, Default, Deserialize, Serialize, ToSchema)]
pub struct UpdateWebhookSubscriptionRequest {
    pub url: Option<String>,
    pub event_types: Option<Vec<ClientEventType>>,
    pub active: Option<bool>,
}

/// Webhook configuration. The signing secret is never returned after creation or rotation.
#[derive(Clone, Debug, Deserialize, Serialize, ToSchema)]
pub struct WebhookSubscription {
    pub id: Uuid,
    pub account_id: Uuid,
    pub wallet_id: Uuid,
    pub url: String,
    pub event_types: Vec<ClientEventType>,
    pub active: bool,
    pub created_at: DateTime<Utc>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub updated_at: Option<DateTime<Utc>>,
}

/// Creation response containing the secret exactly once.
#[derive(Clone, Debug, Deserialize, Serialize, ToSchema)]
pub struct CreatedWebhookSubscription {
    #[serde(flatten)]
    pub subscription: WebhookSubscription,
    /// Base64url secret used to verify `X-SwissKnife-Signature`.
    pub signing_secret: String,
}

/// Secret rotation response. Subsequent attempts use the new secret; an attempt
/// already claimed by a worker may still carry the previous signature.
#[derive(Clone, Debug, Deserialize, Serialize, ToSchema)]
pub struct RotateWebhookSecretResponse {
    pub signing_secret: String,
}

#[derive(Clone, Copy, Debug, Deserialize, Display, EnumString, Eq, PartialEq, Serialize, ToSchema)]
pub enum WebhookDeliveryStatus {
    Pending,
    Delivered,
    Exhausted,
}

/// Delivery state for webhook observability.
#[derive(Clone, Debug, Deserialize, Serialize, ToSchema)]
pub struct WebhookDelivery {
    pub id: Uuid,
    pub subscription_id: Uuid,
    pub event_id: String,
    pub status: WebhookDeliveryStatus,
    pub attempt_count: u32,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub response_status: Option<u16>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub last_error: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub delivered_at: Option<DateTime<Utc>>,
    pub created_at: DateTime<Utc>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub updated_at: Option<DateTime<Utc>>,
}
