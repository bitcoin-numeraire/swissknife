use serde::{Deserialize, Serialize};
use strum_macros::{Display, EnumString};
use utoipa::ToSchema;

use crate::application::entities::Currency;

#[derive(Clone, Debug, Copy, EnumString, Deserialize, Serialize, Display, PartialEq, Eq, Default, ToSchema)]
pub enum BtcNetwork {
    #[default]
    Bitcoin,
    Testnet,
    Testnet4,
    Regtest,
    Simnet,
    Signet,
}

impl From<BtcNetwork> for Currency {
    fn from(network: BtcNetwork) -> Self {
        match network {
            BtcNetwork::Bitcoin => Currency::Bitcoin,
            BtcNetwork::Testnet => Currency::BitcoinTestnet,
            BtcNetwork::Testnet4 => Currency::BitcoinTestnet,
            BtcNetwork::Regtest => Currency::Regtest,
            BtcNetwork::Simnet => Currency::Simnet,
            BtcNetwork::Signet => Currency::Signet,
        }
    }
}

impl From<Currency> for BtcNetwork {
    fn from(currency: Currency) -> Self {
        match currency {
            Currency::Bitcoin => BtcNetwork::Bitcoin,
            Currency::BitcoinTestnet => BtcNetwork::Testnet,
            Currency::Regtest => BtcNetwork::Regtest,
            Currency::Simnet => BtcNetwork::Simnet,
            Currency::Signet => BtcNetwork::Signet,
        }
    }
}
