use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use strum_macros::{Display, EnumString};
use utoipa::ToSchema;
use uuid::Uuid;
#[derive(Clone, Debug)]
#[allow(dead_code)]
pub struct BitcoinAddress {
    pub id: Uuid,
    pub wallet_id: Uuid,
    pub address: String,
    pub used: bool,
    pub address_type: BitcoinAddressType,
    pub created_at: DateTime<Utc>,
    pub updated_at: Option<DateTime<Utc>>,
}

#[derive(Clone, Debug, Copy, EnumString, Deserialize, Serialize, Display, PartialEq, Eq, Default, ToSchema)]
pub enum BitcoinAddressType {
    #[default]
    P2pkh,
    P2sh,
    P2wpkh,
    P2tr,
}
