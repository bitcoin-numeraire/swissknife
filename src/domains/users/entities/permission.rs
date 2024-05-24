use std::str::FromStr;

use tracing::warn;

use crate::application::errors::AuthorizationError;

#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub enum Permission {
    ReadLightningAccounts,
    ReadLightningNode,
    WriteLightningNode,
}

impl FromStr for Permission {
    type Err = AuthorizationError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "read:lightning_accounts" => Ok(Permission::ReadLightningAccounts),
            "read:lightning_node" => Ok(Permission::ReadLightningNode),
            "write:lightning_node" => Ok(Permission::WriteLightningNode),
            // ... handle other permissions ...
            _ => {
                let err = AuthorizationError::ParsePermission(s.to_string());
                warn!("{}", err.to_string());
                Err(err)
            }
        }
    }
}

impl Permission {
    pub fn all_permissions() -> Vec<Self> {
        vec![
            Permission::ReadLightningAccounts,
            Permission::ReadLightningNode,
            Permission::WriteLightningNode,
            // ... include all other permission variants ...
        ]
    }
}
