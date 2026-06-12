use serde::{Deserialize, Serialize};
use serde_with::{serde_as, DisplayFromStr};
use utoipa::{IntoParams, ToSchema};
use uuid::Uuid;

use crate::application::entities::{Ledger, OrderDirection};

pub use swissknife_types::{BtcPayment, InternalPayment, LnPayment, Payment, PaymentStatus};

#[serde_as]
#[derive(Clone, Debug, Deserialize, Serialize, Default, IntoParams, ToSchema)]
pub struct PaymentFilter {
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
    pub status: Option<PaymentStatus>,
    /// Ledger
    pub ledger: Option<Ledger>,

    /// Lightning addresses
    #[schema(example = "donations@numeraire.tech")]
    pub ln_addresses: Option<Vec<String>>,

    /// Bitcoin addresses
    #[schema(example = "bc1q...")]
    pub btc_addresses: Option<Vec<String>>,

    /// Direction of the ordering of results
    #[serde(default)]
    pub order_direction: OrderDirection,
}
