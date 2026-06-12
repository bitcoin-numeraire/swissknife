use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use serde_with::{serde_as, DisplayFromStr};
use strum_macros::{Display, EnumString};
use utoipa::{IntoParams, ToSchema};
use uuid::Uuid;

use crate::{InvoiceStatus, OrderDirection};

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

/// Bitcoin address script type.
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

/// Confirmation status of an on-chain output.
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

/// New Bitcoin Address Request
#[derive(Deserialize, ToSchema)]
pub struct NewBtcAddressRequest {
    /// User ID. Will be populated with your own ID by default
    pub wallet_id: Option<Uuid>,

    /// Address type
    #[serde(rename = "type")]
    pub address_type: Option<BtcAddressType>,
}

/// Bitcoin address query filter.
#[serde_as]
#[derive(Clone, Debug, Deserialize, Serialize, Default, IntoParams)]
pub struct BtcAddressFilter {
    /// Total amount of results to return
    #[serde_as(as = "Option<DisplayFromStr>")]
    pub limit: Option<u64>,
    /// Offset where to start returning results
    #[serde_as(as = "Option<DisplayFromStr>")]
    pub offset: Option<u64>,
    /// List of IDs
    pub ids: Option<Vec<Uuid>>,
    /// Wallet ID. Automatically populated with your ID
    pub wallet_id: Option<Uuid>,
    /// Address
    pub address: Option<String>,
    /// Status
    pub address_type: Option<BtcAddressType>,
    /// Whether the address has been used
    pub used: Option<bool>,

    /// Direction of the ordering of results
    #[serde(default)]
    pub order_direction: OrderDirection,
}
