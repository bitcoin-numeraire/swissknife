use chrono::{DateTime, Utc};
use serde::Serialize;
use utoipa::ToSchema;

use crate::domains::bitcoin::{BitcoinAddress, BitcoinAddressType};

#[derive(Debug, Serialize, ToSchema)]
pub struct BitcoinAddressResponse {
    /// Current deposit address
    pub address: String,

    /// Whether the address has already been used on-chain
    pub used: bool,

    /// Address type
    pub address_type: BitcoinAddressType,

    /// Date of creation in database
    pub created_at: DateTime<Utc>,

    /// Date of update in database
    #[serde(skip_serializing_if = "Option::is_none")]
    pub updated_at: Option<DateTime<Utc>>,
}

impl From<BitcoinAddress> for BitcoinAddressResponse {
    fn from(address: BitcoinAddress) -> Self {
        BitcoinAddressResponse {
            address: address.address,
            used: address.used,
            address_type: address.address_type,
            created_at: address.created_at,
            updated_at: address.updated_at,
        }
    }
}
