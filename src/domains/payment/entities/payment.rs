use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use serde_with::{serde_as, DisplayFromStr};
use strum_macros::{Display, EnumString};
use utoipa::{IntoParams, ToSchema};
use uuid::Uuid;

use crate::{
    application::entities::{Currency, Ledger, OrderDirection},
    domains::bitcoin::BtcOutput,
    domains::lnurl::LnUrlSuccessAction,
};

#[derive(Clone, Debug, Default)]
pub struct Payment {
    pub id: Uuid,
    pub wallet_id: Uuid,
    pub error: Option<String>,
    pub amount_msat: u64,
    pub fee_msat: Option<u64>,
    pub ledger: Ledger,
    pub currency: Currency,
    pub payment_time: Option<DateTime<Utc>>,
    pub status: PaymentStatus,
    pub description: Option<String>,
    pub created_at: DateTime<Utc>,
    pub updated_at: Option<DateTime<Utc>>,
    pub lightning: Option<LnPayment>,
    pub bitcoin: Option<BtcPayment>,
}

#[derive(Clone, Debug, Default)]
pub struct LnPayment {
    pub ln_address: Option<String>,
    pub payment_hash: Option<String>,
    pub payment_preimage: Option<String>,
    pub metadata: Option<String>,
    pub success_action: Option<LnUrlSuccessAction>,
}

#[derive(Clone, Debug, Default)]
pub struct BtcPayment {
    pub destination_address: Option<String>,
    pub txid: Option<String>,
    pub output_id: Option<Uuid>,
    pub output: Option<BtcOutput>,
}

#[derive(Clone, Debug, EnumString, Display, Deserialize, Serialize, PartialEq, Eq, Default, ToSchema)]
pub enum PaymentStatus {
    #[default]
    Pending,
    Settled,
    Failed,
}

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
