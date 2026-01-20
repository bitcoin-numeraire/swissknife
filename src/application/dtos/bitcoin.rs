use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use utoipa::ToSchema;
use uuid::Uuid;

use crate::domains::bitcoin::{BtcAddress, BtcAddressType, BtcNetwork, BtcOutput, BtcOutputStatus};

/// New Invoice Request
#[derive(Deserialize, ToSchema)]
pub struct NewBtcAddressRequest {
    /// User ID. Will be populated with your own ID by default
    pub wallet_id: Option<Uuid>,

    /// Address type
    #[serde(rename = "type")]
    pub address_type: Option<BtcAddressType>,
}

#[derive(Debug, Serialize, ToSchema)]
pub struct BtcAddressResponse {
    /// Internal ID
    pub id: Uuid,
    /// Wallet ID
    pub wallet_id: Uuid,
    /// Current deposit address
    pub address: String,
    /// Whether the address has already been used on-chain
    pub used: bool,
    /// Address type
    pub address_type: crate::domains::bitcoin::BtcAddressType,
    /// Date of creation in database
    pub created_at: DateTime<Utc>,

    /// Date of update in database
    #[serde(skip_serializing_if = "Option::is_none")]
    pub updated_at: Option<DateTime<Utc>>,
}

impl From<BtcAddress> for BtcAddressResponse {
    fn from(address: BtcAddress) -> Self {
        BtcAddressResponse {
            id: address.id,
            wallet_id: address.wallet_id,
            address: address.address,
            used: address.used,
            address_type: address.address_type,
            created_at: address.created_at,
            updated_at: address.updated_at,
        }
    }
}

#[derive(Serialize, ToSchema)]
pub struct BtcOutputResponse {
    /// Internal ID
    pub id: Uuid,
    /// Outpoint
    pub outpoint: String,
    /// Address
    pub address: String,
    /// Amount in satoshis
    pub amount_sat: u64,
    /// Status
    pub status: BtcOutputStatus,
    /// Timestamp
    pub timestamp: DateTime<Utc>,

    /// Block height
    #[serde(skip_serializing_if = "Option::is_none")]
    pub block_height: Option<u32>,

    /// Network
    pub network: BtcNetwork,
    /// Date of creation in database
    pub created_at: DateTime<Utc>,

    /// Date of update in database
    #[serde(skip_serializing_if = "Option::is_none")]
    pub updated_at: Option<DateTime<Utc>>,
}

impl From<BtcOutput> for BtcOutputResponse {
    fn from(output: BtcOutput) -> Self {
        BtcOutputResponse {
            id: output.id,
            outpoint: output.outpoint,
            address: output.address,
            amount_sat: output.amount_sat,
            status: output.status,
            timestamp: output.timestamp,
            block_height: output.block_height,
            network: output.network,
            created_at: output.created_at,
            updated_at: output.updated_at,
        }
    }
}
