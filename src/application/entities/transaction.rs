use serde::{Deserialize, Serialize};
use strum_macros::{Display, EnumString};

#[derive(Clone, Debug, EnumString, Deserialize, Serialize, Display, PartialEq, Eq, Default)]
pub enum Ledger {
    #[default]
    Lightning,
    Internal,
    Onchain,
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
