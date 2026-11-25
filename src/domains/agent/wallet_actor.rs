use super::WalletCapability;
use crate::{
    application::{
        composition::AppServices,
        errors::{ApplicationError, AuthenticationError},
    },
    domains::account::User,
};
use axum::{extract::FromRequestParts, http::request::Parts};
use std::sync::Arc;
use uuid::Uuid;

/// Applied only to wallet operations that explicitly support delegation.
pub enum WalletActor {
    Owner(User),
    Delegated(String),
}

impl FromRequestParts<Arc<AppServices>> for WalletActor {
    type Rejection = ApplicationError;
    async fn from_request_parts(parts: &mut Parts, services: &Arc<AppServices>) -> Result<Self, Self::Rejection> {
        match parts.headers.get("wallet-key") {
            None => Ok(Self::Owner(User::from_request_parts(parts, services).await?)),
            Some(key) => {
                if parts.headers.contains_key("api-key") || parts.headers.contains_key("authorization") {
                    return Err(AuthenticationError::InvalidCredentials.into());
                }
                let value = key.to_str().map_err(|_| AuthenticationError::InvalidCredentials)?;
                Ok(Self::Delegated(value.to_owned()))
            }
        }
    }
}

pub struct WalletAccess {
    pub account_id: Uuid,
    pub agent_key_id: Option<Uuid>,
}

impl WalletActor {
    pub async fn account_for(
        self,
        services: &AppServices,
        wallet: Uuid,
        capability: WalletCapability,
    ) -> Result<Uuid, ApplicationError> {
        Ok(self.authorize(services, wallet, capability).await?.account_id)
    }

    pub async fn authorize(
        self,
        services: &AppServices,
        wallet: Uuid,
        capability: WalletCapability,
    ) -> Result<WalletAccess, ApplicationError> {
        match self {
            Self::Owner(user) => Ok(WalletAccess {
                account_id: user.account_id,
                agent_key_id: None,
            }),
            Self::Delegated(secret) => services.agent_keys.authorize(wallet, &secret, capability).await,
        }
    }
}
