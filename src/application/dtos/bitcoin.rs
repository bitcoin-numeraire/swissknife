use serde::Serialize;
use utoipa::ToSchema;

use crate::domains::bitcoin::BitcoinAddress;

#[derive(Debug, Serialize, ToSchema)]
pub struct BitcoinAddressResponse {
    /// Current deposit address
    pub address: String,
    /// Whether the address has already been used on-chain
    pub used: bool,
}

impl From<BitcoinAddress> for BitcoinAddressResponse {
    fn from(address: BitcoinAddress) -> Self {
        BitcoinAddressResponse {
            address: address.address,
            used: address.used,
        }
    }
}
