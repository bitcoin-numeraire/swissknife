use serde::{Deserialize, Serialize};
use serde_with::{serde_as, DisplayFromStr};
use utoipa::IntoParams;
use uuid::Uuid;

use crate::application::entities::OrderDirection;

pub use swissknife_types::{BtcAddress, BtcAddressType};

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
