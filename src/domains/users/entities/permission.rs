use std::str::FromStr;

use tracing::warn;

use crate::application::errors::AuthorizationError;

#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub enum Permission {
    RegisterLightningAddress,

    ReadLightningNode,
    SendLightningPayment,
}

impl FromStr for Permission {
    type Err = AuthorizationError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "register:lightning_address" => Ok(Permission::RegisterLightningAddress),
            "read:lightning_node" => Ok(Permission::ReadLightningNode),
            "pay:lightning_node" => Ok(Permission::ReadLightningNode),
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
            Permission::RegisterLightningAddress,
            Permission::ReadLightningNode,
            Permission::SendLightningPayment,
            // ... include all other permission variants ...
        ]
    }
}
