use serde::{Deserialize, Serialize};
use strum_macros::{Display, EnumString, VariantNames};
use utoipa::ToSchema;

#[derive(Clone, Debug, PartialEq, Eq, Hash, EnumString, Display, VariantNames, Serialize, Deserialize, ToSchema)]
pub enum Permission {
    #[serde(rename = "read:wallet")]
    ReadWallet,
    #[serde(rename = "write:wallet")]
    WriteWallet,
    #[serde(rename = "read:ln_address")]
    ReadLnAddress,
    #[serde(rename = "write:ln_address")]
    WriteLnAddress,
    #[serde(rename = "read:transaction")]
    ReadLnTransaction,
    #[serde(rename = "write:transaction")]
    WriteLnTransaction,
    #[serde(rename = "read:ln_node")]
    ReadLnNode,
    #[serde(rename = "write:ln_node")]
    WriteLnNode,
    #[serde(rename = "read:api_key")]
    ReadApiKey,
    #[serde(rename = "write:api_key")]
    WriteApiKey,
    #[serde(rename = "read:btc_address")]
    ReadBtcAddress,
    #[serde(rename = "write:btc_address")]
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
