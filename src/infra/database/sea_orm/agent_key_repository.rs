use super::models::{
    prelude::WalletAgentKey,
    wallet_agent_key::{ActiveModel, Column, Model},
};
use crate::{application::errors::DatabaseError, domains::agent::AgentKeyRepository};
use async_trait::async_trait;
use chrono::Utc;
use sea_orm::{ActiveModelTrait, ColumnTrait, DatabaseConnection, EntityTrait, QueryFilter, QueryOrder, Set};
use swissknife_types::{AgentKey, WalletCapabilities};
use uuid::Uuid;

pub struct SeaOrmAgentKeyRepository(pub DatabaseConnection);

impl From<Model> for AgentKey {
    fn from(row: Model) -> Self {
        Self {
            id: row.id,
            wallet_id: row.wallet_id,
            name: row.name,
            capabilities: WalletCapabilities {
                read: row.can_read,
                receive: row.can_receive,
                spend: row.can_spend,
            },
            created_at: row.created_at.and_utc(),
            expires_at: row.expires_at.and_utc(),
        }
    }
}

#[async_trait]
impl AgentKeyRepository for SeaOrmAgentKeyRepository {
    async fn insert(&self, account: Uuid, key: AgentKey, digest: Vec<u8>) -> Result<(), DatabaseError> {
        ActiveModel {
            id: Set(key.id),
            account_id: Set(account),
            wallet_id: Set(key.wallet_id),
            name: Set(key.name),
            key_hash: Set(digest),
            can_read: Set(key.capabilities.read),
            can_receive: Set(key.capabilities.receive),
            can_spend: Set(key.capabilities.spend),
            created_at: Set(key.created_at.naive_utc()),
            expires_at: Set(key.expires_at.naive_utc()),
        }
        .insert(&self.0)
        .await
        .map_err(|err| DatabaseError::Insert(err.to_string()))?;
        Ok(())
    }
    async fn list(&self, account: Uuid, wallet: Uuid) -> Result<Vec<AgentKey>, DatabaseError> {
        WalletAgentKey::find()
            .filter(Column::AccountId.eq(account))
            .filter(Column::WalletId.eq(wallet))
            .order_by_desc(Column::CreatedAt)
            .order_by_asc(Column::Id)
            .all(&self.0)
            .await
            .map(|rows| rows.into_iter().map(Into::into).collect())
            .map_err(|err| DatabaseError::FindMany(err.to_string()))
    }
    async fn revoke(&self, account: Uuid, wallet: Uuid, key: Uuid) -> Result<bool, DatabaseError> {
        WalletAgentKey::delete_by_id(key)
            .filter(Column::AccountId.eq(account))
            .filter(Column::WalletId.eq(wallet))
            .exec(&self.0)
            .await
            .map(|result| result.rows_affected == 1)
            .map_err(|err| DatabaseError::Delete(err.to_string()))
    }
    async fn active_grant(
        &self,
        wallet: Uuid,
        digest: Vec<u8>,
    ) -> Result<Option<(Uuid, WalletCapabilities)>, DatabaseError> {
        let row = WalletAgentKey::find()
            .filter(Column::KeyHash.eq(digest))
            .filter(Column::WalletId.eq(wallet))
            .filter(Column::ExpiresAt.gt(Utc::now().naive_utc()))
            .one(&self.0)
            .await
            .map_err(|err| DatabaseError::FindOne(err.to_string()))?;
        Ok(row.map(|row| {
            (
                row.account_id,
                WalletCapabilities {
                    read: row.can_read,
                    receive: row.can_receive,
                    spend: row.can_spend,
                },
            )
        }))
    }
}
