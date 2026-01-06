use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use strum_macros::{Display, EnumString};
use utoipa::ToSchema;
use uuid::Uuid;

#[derive(Clone, Debug)]
pub struct BitcoinAddress {
    pub id: Uuid,
    pub wallet_id: Uuid,
    pub address: String,
    pub used: bool,
    pub address_type: BitcoinAddressType,
    pub created_at: DateTime<Utc>,
    pub updated_at: Option<DateTime<Utc>>,
}

#[derive(Clone, Debug, Copy, Deserialize, Serialize, EnumString, Display, PartialEq, Eq, Default, ToSchema)]
#[serde(rename_all = "lowercase")]
#[strum(serialize_all = "lowercase")]
pub enum BitcoinAddressType {
    P2pkh,
    P2sh,
    #[default]
    P2wpkh,
    P2tr,
}
