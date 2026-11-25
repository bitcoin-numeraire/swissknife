use super::AgentKeyRepository;
use crate::application::errors::{ApplicationError, AuthenticationError, AuthorizationError, DataError};
use async_trait::async_trait;
use bitcoin::hashes::{sha256, Hash};
use chrono::{Duration, Utc};
use std::sync::Arc;
use swissknife_types::{AgentKey, CreateAgentKey, IssuedAgentKey};
use uuid::Uuid;

#[derive(Clone, Copy)]
pub enum WalletCapability {
    Read,
    Receive,
    Spend,
}

#[cfg_attr(test, mockall::automock)]
#[async_trait]
pub trait AgentKeyUseCases: Send + Sync {
    async fn issue(
        &self,
        owner: Uuid,
        wallet: Uuid,
        request: CreateAgentKey,
    ) -> Result<IssuedAgentKey, ApplicationError>;
    async fn list(&self, owner: Uuid, wallet: Uuid) -> Result<Vec<AgentKey>, ApplicationError>;
    async fn revoke(&self, owner: Uuid, wallet: Uuid, key: Uuid) -> Result<(), ApplicationError>;
    async fn authorize(
        &self,
        wallet: Uuid,
        secret: &str,
        capability: WalletCapability,
    ) -> Result<Uuid, ApplicationError>;
}

pub struct AgentKeyService(pub Arc<dyn AgentKeyRepository>);

#[async_trait]
impl AgentKeyUseCases for AgentKeyService {
    async fn issue(
        &self,
        owner: Uuid,
        wallet: Uuid,
        request: CreateAgentKey,
    ) -> Result<IssuedAgentKey, ApplicationError> {
        let name = request.name.trim();
        let now = Utc::now();
        if name.is_empty() || name.chars().count() > 100 {
            return Err(DataError::Validation("Name must contain 1 to 100 characters".into()).into());
        }
        if request.expires_at <= now || request.expires_at > now + Duration::days(365) {
            return Err(DataError::Validation("Choose an expiry within the next 365 days".into()).into());
        }
        let caps = request.capabilities;
        if !(caps.read || caps.receive || caps.spend) {
            return Err(DataError::Validation("Select at least one wallet capability".into()).into());
        }
        let bytes: [u8; 32] = rand::random();
        let metadata = AgentKey {
            id: Uuid::new_v4(),
            wallet_id: wallet,
            name: name.to_owned(),
            capabilities: caps,
            expires_at: request.expires_at,
            created_at: now,
        };
        self.0
            .insert(
                owner,
                metadata.clone(),
                sha256::Hash::hash(&bytes).to_byte_array().to_vec(),
            )
            .await?;
        Ok(IssuedAgentKey {
            metadata,
            key: format!("swka_{}", hex::encode(bytes)),
        })
    }
    async fn list(&self, owner: Uuid, wallet: Uuid) -> Result<Vec<AgentKey>, ApplicationError> {
        Ok(self.0.list(owner, wallet).await?)
    }
    async fn revoke(&self, owner: Uuid, wallet: Uuid, key: Uuid) -> Result<(), ApplicationError> {
        if !self.0.revoke(owner, wallet, key).await? {
            return Err(DataError::NotFound("Wallet credential not found".into()).into());
        }
        Ok(())
    }
    async fn authorize(
        &self,
        wallet: Uuid,
        secret: &str,
        capability: WalletCapability,
    ) -> Result<Uuid, ApplicationError> {
        let mut bytes = [0; 32];
        let encoded = secret
            .strip_prefix("swka_")
            .ok_or(AuthenticationError::InvalidCredentials)?;
        hex::decode_to_slice(encoded, &mut bytes).map_err(|_| AuthenticationError::InvalidCredentials)?;
        let digest = sha256::Hash::hash(&bytes).to_byte_array().to_vec();
        let (account, caps) = self
            .0
            .active_grant(wallet, digest)
            .await?
            .ok_or(AuthenticationError::InvalidCredentials)?;
        let allowed = match capability {
            WalletCapability::Read => caps.read,
            WalletCapability::Receive => caps.receive,
            WalletCapability::Spend => caps.spend,
        };
        if !allowed {
            return Err(AuthorizationError::WalletCapability.into());
        }
        Ok(account)
    }
}
