use serde::{Deserialize, Serialize};
use strum_macros::{Display, EnumString};
use utoipa::ToSchema;

/// The ledger a transaction settles on.
#[derive(Clone, Debug, EnumString, Deserialize, Serialize, Display, PartialEq, Eq, Default, ToSchema)]
pub enum Ledger {
    #[default]
    Lightning,
    Internal,
    Onchain,
}

/// The currency and network a transaction is denominated in.
#[derive(Clone, Debug, EnumString, Deserialize, Serialize, Display, PartialEq, Eq, Default, ToSchema)]
pub enum Currency {
    #[default]
    Bitcoin,
    BitcoinTestnet,
    Regtest,
    Simnet,
    Signet,
}
