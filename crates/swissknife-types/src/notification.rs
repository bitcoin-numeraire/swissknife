use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use utoipa::{IntoParams, ToSchema};
use uuid::Uuid;

use crate::{ClientEventType, Ledger};

/// Presentation state for one terminal payment or receipt belonging to the signed-in account.
#[derive(Clone, Debug, Deserialize, Serialize, ToSchema)]
pub struct AccountNotification {
    pub source_event_id: i32,
    pub event_type: ClientEventType,
    pub wallet_id: Uuid,
    pub resource_id: Uuid,
    pub created_at: DateTime<Utc>,
    pub ledger: Option<Ledger>,
    /// Decimal integer string so JavaScript clients can represent the full amount.
    pub amount_msat: Option<String>,
    pub description: Option<String>,
    pub is_read: bool,
    pub archived: bool,
}

#[derive(Debug, Deserialize, Serialize, ToSchema)]
pub struct NotificationPage {
    pub items: Vec<AccountNotification>,
    /// Number of unread items in the active inbox, across all pages.
    pub unread_count: u64,
    /// Exclusive cursor for the next page, ordered newest first.
    pub next_before: Option<i32>,
}

#[derive(Default, Debug, Deserialize, Serialize, IntoParams, ToSchema)]
#[into_params(parameter_in = Query)]
pub struct NotificationQuery {
    pub before: Option<i32>,
    pub limit: Option<u32>,
    #[serde(default)]
    pub archived: bool,
}

#[derive(Clone, Copy, Debug, Deserialize, Serialize, ToSchema)]
#[serde(rename_all = "snake_case")]
pub enum NotificationAction {
    Read,
    Archive,
}

#[derive(Debug, Deserialize, Serialize, ToSchema)]
pub struct UpdateNotificationRequest {
    pub action: NotificationAction,
}

#[derive(Debug, Deserialize, Serialize, ToSchema)]
pub struct ReadNotificationsRequest {
    /// Mark notifications through the newest ID observed by the caller.
    pub through: i32,
}
