use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use serde_with::{serde_as, DisplayFromStr};
use strum_macros::{Display, EnumString};
use utoipa::{IntoParams, ToSchema};
use uuid::Uuid;

use crate::application::entities::OrderDirection;

#[derive(Clone, Debug)]
pub struct BtcAddress {
    pub id: Uuid,
    pub wallet_id: Uuid,
    pub address: String,
    pub used: bool,
    pub address_type: BtcAddressType,
    pub created_at: DateTime<Utc>,
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