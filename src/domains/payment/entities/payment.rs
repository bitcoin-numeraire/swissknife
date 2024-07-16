use breez_sdk_core::SuccessActionProcessed;
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use serde_with::{serde_as, DisplayFromStr};
use strum_macros::{Display, EnumString};
use utoipa::{IntoParams, ToSchema};
use uuid::Uuid;

use crate::application::entities::{Ledger, OrderDirection};

#[derive(Clone, Debug, Default)]
pub struct Payment {
    pub id: Uuid,
    pub wallet_id: Uuid,
    pub ln_address: Option<String>,
    pub payment_hash: Option<String>,
    pub payment_preimage: Option<String>,
    pub error: Option<String>,
    pub amount_msat: u64,
    pub fee_msat: Option<u64>,
    pub ledger: Ledger,
    pub payment_time: Option<DateTime<Utc>>,
    pub status: PaymentStatus,
    pub description: Option<String>,
    pub metadata: Option<String>,
    pub success_action: Option<SuccessActionProcessed>,
    pub created_at: DateTime<Utc>,
    pub updated_at: Option<DateTime<Utc>>,
}

#[derive(
    Clone, Debug, EnumString, Display, Deserialize, Serialize, PartialEq, Eq, Default, ToSchema,
)]
pub enum PaymentStatus {
    #[default]
    Pending,
    Settled,
    Failed,
}

#[serde_as]
#[derive(Clone, Debug, Deserialize, Serialize, Default, IntoParams)]
pub struct PaymentFilter {
    /// Total amount of results to return
    #[serde_as(as = "Option<DisplayFromStr>")]
    pub limit: Option<u64>,
    /// Offset where to start returning results
    #[serde_as(as = "Option<DisplayFromStr>")]
    pub offset: Option<u64>,
    /// List of IDs
    pub ids: Option<Vec<Uuid>>,
    /// User ID. Automatically populated with your user ID
    pub wallet_id: Option<String>,
    /// Status
    pub status: Option<PaymentStatus>,
    /// Ledger
    pub ledger: Option<Ledger>,
    /// Direction of the ordering of results
    #[serde(default)]
    pub order_direction: OrderDirection,
}
