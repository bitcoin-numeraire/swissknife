use serde::{Deserialize, Serialize};
use utoipa::ToSchema;

/// An API access scope granted to a JWT or API key.
#[derive(Clone, Debug, PartialEq, Eq, Hash, Serialize, Deserialize, ToSchema)]
pub enum Permission {
    #[serde(rename = "read:account")]
    ReadAccount,
    #[serde(rename = "write:account")]
    WriteAccount,
    #[serde(rename = "read:wallet")]
    ReadWallet,
    #[serde(rename = "write:wallet")]
    WriteWallet,
    #[serde(rename = "read:ln_address")]
    ReadLnAddress,
    #[serde(rename = "write:ln_address")]
    WriteLnAddress,
    #[serde(rename = "read:transaction")]
    ReadTransaction,
    #[serde(rename = "write:transaction")]
    WriteTransaction,
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
            Permission::ReadAccount,
            Permission::WriteAccount,
            Permission::ReadWallet,
            Permission::WriteWallet,
            Permission::ReadLnAddress,
            Permission::WriteLnAddress,
            Permission::ReadTransaction,
            Permission::WriteTransaction,
            Permission::ReadLnNode,
            Permission::WriteLnNode,
            Permission::ReadApiKey,
            Permission::WriteApiKey,
            Permission::ReadBtcAddress,
            Permission::WriteBtcAddress,
        ]
    }
}
