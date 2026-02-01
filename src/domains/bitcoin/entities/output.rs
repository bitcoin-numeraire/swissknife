use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use strum_macros::{Display, EnumString};
use utoipa::ToSchema;
use uuid::Uuid;

use crate::domains::invoice::InvoiceStatus;

#[derive(Clone, Debug, Default)]
pub struct BtcOutput {
    pub id: Uuid,
    pub outpoint: String,
    pub txid: String,
    pub output_index: u32,
    pub address: String,
    pub amount_sat: u64,
    pub status: BtcOutputStatus,
    pub block_height: Option<u32>,
    pub created_at: DateTime<Utc>,
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
