use serde::{Deserialize, Serialize};
use serde_with::{serde_as, DisplayFromStr};
use utoipa::{IntoParams, ToSchema};
use uuid::Uuid;

use crate::application::entities::{Ledger, OrderDirection};

pub use swissknife_api_types::{Invoice, InvoiceStatus, LnInvoice};

#[serde_as]
#[derive(Clone, Debug, Deserialize, Serialize, Default, IntoParams, ToSchema)]
pub struct InvoiceFilter {
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
    /// Status
    pub status: Option<InvoiceStatus>,
    /// Ledger
    pub ledger: Option<Ledger>,
    /// Order by
    #[serde(default)]
    pub order_by: InvoiceOrderBy,
    /// Direction of the ordering of results
    #[serde(default)]
    pub order_direction: OrderDirection,
}

#[derive(Clone, Debug, Deserialize, Serialize, PartialEq, Eq, Default, ToSchema)]
pub enum InvoiceOrderBy {
    #[default]
    CreatedAt,
    PaymentTime,
    UpdatedAt,
}
