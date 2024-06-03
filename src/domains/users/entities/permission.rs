use std::str::FromStr;

use tracing::warn;

use crate::application::errors::AuthorizationError;

#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub enum Permission {
    ReadLnAddress,
    WriteLnAddress,
    ReadLnTransaction,
    WriteLnTransaction,
    ReadLnNode,
    WriteLnNode,
}

impl FromStr for Permission {
    type Err = AuthorizationError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "read:ln_address" => Ok(Permission::ReadLnAddress),
            "write:ln_address" => Ok(Permission::WriteLnAddress),
            "read:transaction" => Ok(Permission::ReadLnTransaction),
            "write:transaction" => Ok(Permission::WriteLnTransaction),
            "read:ln_node" => Ok(Permission::ReadLnNode),
            "write:ln_node" => Ok(Permission::WriteLnNode),
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
            Permission::ReadLnAddress,
            Permission::WriteLnAddress,
            Permission::ReadLnTransaction,
            Permission::WriteLnTransaction,
            Permission::ReadLnNode,
            Permission::WriteLnNode,
            // ... include all other permission variants ...
        ]
    }
}
