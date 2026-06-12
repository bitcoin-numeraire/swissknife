use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use strum_macros::{Display, EnumString};
use utoipa::ToSchema;
use uuid::Uuid;

use crate::InvoiceStatus;

/// Bitcoin Address
#[derive(Clone, Debug, Serialize, ToSchema)]
pub struct BtcAddress {
    /// Internal ID
    pub id: Uuid,
    /// Wallet ID
    pub wallet_id: Uuid,
    /// Current deposit address
    pub address: String,
    /// Whether the address has already been used on-chain
    pub used: bool,
    /// Address type
    pub address_type: BtcAddressType,
    /// Date of creation in database
    pub created_at: DateTime<Utc>,
    /// Date of update in database
    #[serde(skip_serializing_if = "Option::is_none")]
    pub updated_at: Option<DateTime<Utc>>,
}

#[derive(Clone, Debug, Copy, Deserialize, Serialize, EnumString, Display, PartialEq, Eq, Default, ToSchema)]
#[serde(rename_all = "lowercase")]
#[strum(serialize_all = "lowercase")]
pub enum BtcAddressType {
    P2pkh,
    P2sh,
    #[default]
    P2wpkh,
    P2tr,
}

/// Bitcoin Output
#[derive(Clone, Debug, Default, Serialize, ToSchema)]
pub struct BtcOutput {
    /// Internal ID
    pub id: Uuid,
    /// Outpoint
    pub outpoint: String,
    /// Transaction ID. Internal only.
    #[serde(skip)]
    pub txid: String,
    /// Output index. Internal only.
    #[serde(skip)]
    pub output_index: u32,
    /// Address
    pub address: String,
    /// Amount in satoshis
    pub amount_sat: u64,
    /// Status
    pub status: BtcOutputStatus,
    /// Block height
    #[serde(skip_serializing_if = "Option::is_none")]
    pub block_height: Option<u32>,
    /// Date of creation in database
    pub created_at: DateTime<Utc>,
    /// Date of update in database
    #[serde(skip_serializing_if = "Option::is_none")]
    pub updated_at: Option<DateTime<Utc>>,
}

#[derive(Clone, Debug, Copy, EnumString, Deserialize, Serialize, Display, PartialEq, Eq, Default, ToSchema)]
pub enum BtcOutputStatus {
    #[default]
    Unconfirmed,
    Confirmed,
    Spent,
    Immature,
}

impl From<BtcOutputStatus> for InvoiceStatus {
    fn from(status: BtcOutputStatus) -> Self {
        match status {
            BtcOutputStatus::Unconfirmed => InvoiceStatus::Pending,
            BtcOutputStatus::Confirmed => InvoiceStatus::Settled,
            BtcOutputStatus::Spent => InvoiceStatus::Settled,
            BtcOutputStatus::Immature => InvoiceStatus::Pending,
        }
    }
}
