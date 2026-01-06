use chrono::{DateTime, Utc};
use std::str::FromStr;
use uuid::Uuid;

use crate::application::errors::DataError;
#[derive(Clone, Debug)]
#[allow(dead_code)]
pub struct BitcoinAddress {
    pub id: Uuid,
    pub wallet_id: Uuid,
    pub address: String,
    pub used: bool,
    pub created_at: DateTime<Utc>,
    pub updated_at: Option<DateTime<Utc>>,
}

#[derive(Clone, Debug, Default, Copy)]
pub enum BitcoinAddressType {
    #[default]
    P2pkh,
    P2sh,
    P2wpkh,
    P2tr,
}

impl FromStr for BitcoinAddressType {
    type Err = DataError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.to_lowercase().as_str() {
            "p2pkh" => Ok(BitcoinAddressType::P2pkh),
            "p2sh" => Ok(BitcoinAddressType::P2sh),
            "p2wpkh" | "bech32" => Ok(BitcoinAddressType::P2wpkh),
            "p2tr" => Ok(BitcoinAddressType::P2tr),
            _ => Err(DataError::Validation(format!("Invalid address type: {}", s))),
        }
    }
}
