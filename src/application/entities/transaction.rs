use serde::{Deserialize, Serialize};
use strum_macros::{Display, EnumString};

#[derive(Clone, Debug, EnumString, Deserialize, Serialize, Display, PartialEq, Eq, Default)]
pub enum Ledger {
    #[default]
    LIGHTNING,
    INTERNAL,
    ONCHAIN,
}

#[derive(Clone, Debug, EnumString, Deserialize, Serialize, Display, PartialEq, Eq, Default)]
pub enum Currency {
    #[default]
    Bitcoin,
    BitcoinTestnet,
    Regtest,
    Simnet,
    Signet,
}
