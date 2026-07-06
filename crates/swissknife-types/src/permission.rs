use serde::{Deserialize, Serialize};
use strum_macros::{Display, EnumString};
use utoipa::ToSchema;

/// An API access scope granted to a JWT or API key.
#[derive(Clone, Debug, PartialEq, Eq, Hash, EnumString, Display, Serialize, Deserialize, ToSchema)]
pub enum Permission {
    #[serde(rename = "read:wallet")]
    #[strum(serialize = "read:wallet")]
    ReadWallet,
    #[serde(rename = "write:wallet")]
    #[strum(serialize = "write:wallet")]
    WriteWallet,
    #[serde(rename = "read:ln_address")]
    #[strum(serialize = "read:ln_address")]
    ReadLnAddress,
    #[serde(rename = "write:ln_address")]
    #[strum(serialize = "write:ln_address")]
    WriteLnAddress,
    #[serde(rename = "read:transaction")]
    #[strum(serialize = "read:transaction")]
    ReadLnTransaction,
    #[serde(rename = "write:transaction")]
    #[strum(serialize = "write:transaction")]
    WriteLnTransaction,
    #[serde(rename = "read:ln_node")]
    #[strum(serialize = "read:ln_node")]
    ReadLnNode,
    #[serde(rename = "write:ln_node")]
    #[strum(serialize = "write:ln_node")]
    WriteLnNode,
    #[serde(rename = "read:api_key")]
    #[strum(serialize = "read:api_key")]
    ReadApiKey,
    #[serde(rename = "write:api_key")]
    #[strum(serialize = "write:api_key")]
    WriteApiKey,
    #[serde(rename = "read:btc_address")]
    #[strum(serialize = "read:btc_address")]
    ReadBtcAddress,
    #[serde(rename = "write:btc_address")]
    #[strum(serialize = "write:btc_address")]
    WriteBtcAddress,
}

impl Permission {
    pub fn all_permissions() -> Vec<Self> {
        vec![
            Permission::ReadWallet,
            Permission::WriteWallet,
            Permission::ReadLnAddress,
            Permission::WriteLnAddress,
            Permission::ReadLnTransaction,
            Permission::WriteLnTransaction,
            Permission::ReadLnNode,
            Permission::WriteLnNode,
            Permission::ReadApiKey,
            Permission::WriteApiKey,
            Permission::ReadBtcAddress,
            Permission::WriteBtcAddress,
        ]
    }
}
