use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use utoipa::{IntoParams, ToSchema};
use uuid::Uuid;

#[derive(Clone, Debug, Serialize, Deserialize, ToSchema)]
pub struct CreateOfferRequest {
    /// Required for administration. Account endpoints use the wallet in the path.
    pub wallet_id: Option<Uuid>,
    /// Reuse when retrying a single creation.
    pub idempotency_key: Uuid,
    pub description: String,
    /// Omit to let each payer choose the amount.
    pub amount_msat: Option<u64>,
    #[serde(default)]
    pub single_use: bool,
}

#[derive(
    Clone, Copy, Debug, Serialize, Deserialize, PartialEq, Eq, ToSchema, strum_macros::Display, strum_macros::EnumString,
)]
#[serde(rename_all = "snake_case")]
#[strum(serialize_all = "snake_case")]
pub enum OfferState {
    Creating,
    Active,
    Disabling,
    Disabled,
}

#[derive(Clone, Debug, Serialize, Deserialize, ToSchema)]
pub struct WalletOffer {
    pub id: Uuid,
    pub wallet_id: Uuid,
    pub description: String,
    pub amount_msat: Option<u64>,
    pub single_use: bool,
    pub state: OfferState,
    /// Available once the original node has registered this offer.
    pub bolt12: Option<String>,
    pub provider_id: Option<String>,
    pub created_at: DateTime<Utc>,
}

#[derive(Clone, Debug, Serialize, Deserialize, ToSchema)]
pub struct OfferInvoice {
    pub offer_id: Uuid,
    pub payment_hash: String,
    pub bolt12: String,
    pub payer_note: Option<String>,
}

#[derive(Default, Debug, Deserialize, IntoParams)]
pub struct OfferFilter {
    pub wallet_id: Option<Uuid>,
    pub offset: Option<u64>,
    pub limit: Option<u64>,
}
