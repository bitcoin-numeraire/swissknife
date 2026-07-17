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
