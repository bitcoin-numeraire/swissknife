use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use serde_with::serde_as;
use strum_macros::{Display, EnumString};
use utoipa::{IntoParams, ToSchema};
use uuid::Uuid;

use crate::domains::bitcoin::{BitcoinAddress, BitcoinNetwork, BitcoinOutput, BitcoinOutputStatus};

#[derive(Debug, Serialize, ToSchema)]
pub struct BitcoinAddressResponse {
    /// Internal ID
    pub id: Uuid,
    /// Current deposit address
    pub address: String,
    /// Whether the address has already been used on-chain
    pub used: bool,
    /// Address type
    pub address_type: crate::domains::bitcoin::BitcoinAddressType,
    /// Date of creation in database
    pub created_at: DateTime<Utc>,

    /// Date of update in database
    #[serde(skip_serializing_if = "Option::is_none")]
    pub updated_at: Option<DateTime<Utc>>,
}

impl From<BitcoinAddress> for BitcoinAddressResponse {
    fn from(address: BitcoinAddress) -> Self {
        BitcoinAddressResponse {
            id: address.id,
            address: address.address,
            used: address.used,
            address_type: address.address_type,
            created_at: address.created_at,
            updated_at: address.updated_at,
        }
    }
}

#[derive(Clone, Copy, Debug, Deserialize, Serialize, EnumString, Display, PartialEq, Eq, Default, ToSchema)]
#[serde(rename_all = "lowercase")]
#[strum(serialize_all = "lowercase")]
pub enum BitcoinAddressType {
    #[default]
    P2wpkh,
    P2tr,
}

impl From<BitcoinAddressType> for crate::domains::bitcoin::BitcoinAddressType {
    fn from(dto: BitcoinAddressType) -> Self {
        match dto {
            BitcoinAddressType::P2wpkh => crate::domains::bitcoin::BitcoinAddressType::P2wpkh,
            BitcoinAddressType::P2tr => crate::domains::bitcoin::BitcoinAddressType::P2tr,
        }
    }
}

#[serde_as]
#[derive(Clone, Debug, Deserialize, Serialize, Default, IntoParams)]
pub struct BitcoinAddressQueryParams {
    /// Address type
    #[serde(rename = "type")]
    pub address_type: Option<BitcoinAddressType>,
}

#[derive(Serialize, ToSchema)]
pub struct BitcoinOutputResponse {
    /// Internal ID
    pub id: Uuid,
    /// Outpoint
    pub outpoint: String,
    /// TXID
    pub txid: String,
    /// Output index
    pub output_index: u32,
    /// Address
    pub address: String,
    /// Amount in satoshis
    pub amount_sat: u64,
    /// Status
    pub status: BitcoinOutputStatus,
    /// Timestamp
    pub timestamp: DateTime<Utc>,
    /// Block height
    pub block_height: u32,
    /// Network
    pub network: BitcoinNetwork,
    /// Date of creation in database
    pub created_at: DateTime<Utc>,

    /// Date of update in database
    #[serde(skip_serializing_if = "Option::is_none")]
    pub updated_at: Option<DateTime<Utc>>,
}

impl From<BitcoinOutput> for BitcoinOutputResponse {
    fn from(output: BitcoinOutput) -> Self {
        BitcoinOutputResponse {
            id: output.id,
            outpoint: output.outpoint,
            txid: output.txid,
            output_index: output.output_index,
            address: output.address.unwrap(),
            amount_sat: output.amount_sat,
            status: output.status,
            timestamp: output.timestamp.unwrap(),
            block_height: output.block_height.unwrap(),
            network: output.network,
            created_at: output.created_at,
            updated_at: output.updated_at,
        }
    }
}
