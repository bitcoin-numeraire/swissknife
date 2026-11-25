use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use serde_with::{serde_as, DisplayFromStr};
use strum_macros::{Display, EnumString};
use utoipa::ToSchema;
use uuid::Uuid;

#[derive(Clone, Copy, Debug, Deserialize, Serialize, PartialEq, Eq, Display, EnumString, ToSchema)]
#[serde(rename_all = "snake_case")]
#[strum(serialize_all = "snake_case")]
pub enum CreditPurpose {
    Opening,
    TopUp,
}

#[serde_as]
#[derive(Clone, Debug, Serialize, Deserialize, ToSchema, PartialEq, Eq)]
pub struct CreateWalletCredit {
    /// Client-generated replay key. Keep this UUID when retrying the same operation.
    pub request_id: Uuid,
    pub account_id: Uuid,
    pub asset_id: Uuid,
    /// Positive integer in the selected asset's smallest units, encoded as a decimal string.
    #[serde_as(as = "DisplayFromStr")]
    #[schema(value_type = String, example = "1000000")]
    pub amount_units: u64,
    pub purpose: CreditPurpose,
    pub reason: String,
}

#[derive(Clone, Debug, Serialize, Deserialize, ToSchema, PartialEq, Eq)]
pub struct WalletCreditReceipt {
    #[serde(flatten)]
    pub request: CreateWalletCredit,
    pub wallet_id: Uuid,
    pub invoice_id: Uuid,
    pub actor_account_id: Uuid,
    pub api_key_id: Option<Uuid>,
    pub created_at: DateTime<Utc>,
}
