use serde::{Deserialize, Serialize};
use serde_with::{serde_as, DisplayFromStr};
use utoipa::IntoParams;
use uuid::Uuid;

use crate::application::entities::OrderDirection;

pub use swissknife_types::LnAddress;

#[serde_as]
#[derive(Clone, Debug, Deserialize, Serialize, Default, IntoParams)]
pub struct LnAddressFilter {
    /// Total amount of results to return
    #[serde_as(as = "Option<DisplayFromStr>")]
    pub limit: Option<u64>,
    /// Offset where to start returning results
    #[serde_as(as = "Option<DisplayFromStr>")]
    pub offset: Option<u64>,
    /// List of IDs
    pub ids: Option<Vec<Uuid>>,
    /// wallet ID. Automatically populated with your ID
    pub wallet_id: Option<Uuid>,
    /// Username
    pub username: Option<String>,
    /// Active
    pub active: Option<bool>,
    /// Direction of the ordering of results
    #[serde(default)]
    pub order_direction: OrderDirection,
}
