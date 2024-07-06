use breez_sdk_core::SuccessActionProcessed;
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use serde_with::{serde_as, DisplayFromStr};
use strum_macros::{Display, EnumString};
use utoipa::{IntoParams, ToSchema};
use uuid::Uuid;

use crate::application::entities::Ledger;

#[derive(Clone, Debug, Default, Serialize, ToSchema)]
pub struct Payment {
    /// Internal ID
    pub id: Uuid,
    /// User ID
    pub user_id: String,

    /// Lightning Address. Populated when sending to a LN Address
    #[schema(example = "hello@numeraire.tech")]
    pub ln_address: Option<String>,

    /// Payment hash
    #[serde(skip_serializing_if = "Option::is_none")]
    #[schema(example = "b587c7f76339e3fb87ad2b...")]
    pub payment_hash: Option<String>,

    /// Payment Preimage
    #[serde(skip_serializing_if = "Option::is_none")]
    pub payment_preimage: Option<String>,

    /// Error message
    #[serde(skip_serializing_if = "Option::is_none")]
    #[schema(example = "failed to pay error message")]
    pub error: Option<String>,

    /// Amount in millisatoshis.
    pub amount_msat: u64,
    #[serde(skip_serializing_if = "Option::is_none")]
    /// Fees paid. Populated when a new channel is opened to receive the funds
    pub fee_msat: Option<u64>,
    /// Ledger
    pub ledger: Ledger,
    /// Payment time
    #[serde(skip_serializing_if = "Option::is_none")]
    pub payment_time: Option<DateTime<Utc>>,
    /// Status
    pub status: PaymentStatus,
    /// Description
    #[serde(skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
    /// Metadata
    #[serde(skip_serializing_if = "Option::is_none")]
    pub metadata: Option<String>,
    /// Success Action. Populated when sending to a LNURL or LN Address
    #[serde(skip_serializing_if = "Option::is_none")]
    pub success_action: Option<SuccessActionProcessed>,
    /// Date of creation in database
    pub created_at: DateTime<Utc>,
    /// Date of update in database
    #[serde(skip_serializing_if = "Option::is_none")]
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
    pub user_id: Option<String>,
    /// Status
    pub status: Option<PaymentStatus>,
}
