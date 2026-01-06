use serde::{Deserialize, Serialize};
use strum_macros::{Display, EnumString};
use utoipa::ToSchema;

use crate::application::entities::Currency;

#[derive(Clone, Debug, Copy, EnumString, Deserialize, Serialize, Display, PartialEq, Eq, Default, ToSchema)]
pub enum BitcoinNetwork {
    #[default]
    Bitcoin,
    Testnet,
    Testnet4,
    Regtest,
    Simnet,
    Signet,
}

impl From<BitcoinNetwork> for Currency {
    fn from(network: BitcoinNetwork) -> Self {
        match network {
            BitcoinNetwork::Bitcoin => Currency::Bitcoin,
            BitcoinNetwork::Testnet => Currency::BitcoinTestnet,
            BitcoinNetwork::Testnet4 => Currency::BitcoinTestnet,
            BitcoinNetwork::Regtest => Currency::Regtest,
            BitcoinNetwork::Simnet => Currency::Simnet,
            BitcoinNetwork::Signet => Currency::Signet,
        }
    }
}
