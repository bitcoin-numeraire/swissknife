use serde::{Deserialize, Serialize};
use strum_macros::{Display, EnumString, VariantNames};

#[derive(
    Clone, Debug, PartialEq, Eq, Hash, EnumString, Display, VariantNames, Serialize, Deserialize,
)]
pub enum Permission {
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
}

impl Permission {
    pub fn all_permissions() -> Vec<Self> {
        vec![
            Permission::ReadLnAddress,
            Permission::WriteLnAddress,
            Permission::ReadLnTransaction,
            Permission::WriteLnTransaction,
            Permission::ReadLnNode,
            Permission::WriteLnNode,
        ]
    }
}
