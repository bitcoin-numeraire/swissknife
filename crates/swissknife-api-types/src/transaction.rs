use serde::{Deserialize, Serialize};
use strum_macros::{Display, EnumString};
use utoipa::ToSchema;

#[derive(Clone, Debug, EnumString, Deserialize, Serialize, Display, PartialEq, Eq, Default, ToSchema)]
pub enum Ledger {
    #[default]
    Lightning,
    Internal,
    Onchain,
}

#[derive(Clone, Debug, EnumString, Deserialize, Serialize, Display, PartialEq, Eq, Default, ToSchema)]
pub enum Currency {
    #[default]
    Bitcoin,
    BitcoinTestnet,
    Regtest,
    Simnet,
    Signet,
}
