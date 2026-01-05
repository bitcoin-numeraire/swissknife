use async_trait::async_trait;
use chrono::Utc;
use sea_orm::{
    ActiveModelTrait, ColumnTrait, DatabaseConnection, EntityTrait, QueryFilter, QueryOrder, Set, Unchanged,
};
use uuid::Uuid;

use crate::{
    application::errors::DatabaseError,
    domains::bitcoin::{BitcoinAddress, BitcoinAddressRepository},
    infra::database::sea_orm::models::btc_address::{ActiveModel, Column, Entity},
};

#[derive(Clone)]
pub struct SeaOrmBitcoinAddressRepository {
    pub db: DatabaseConnection,
}

impl SeaOrmBitcoinAddressRepository {
    pub fn new(db: DatabaseConnection) -> Self {
        Self { db }
    }
}

#[async_trait]
impl BitcoinAddressRepository for SeaOrmBitcoinAddressRepository {
    async fn find_by_wallet_unused(&self, wallet_id: Uuid) -> Result<Option<BitcoinAddress>, DatabaseError> {
        let model = Entity::find()
            .filter(Column::WalletId.eq(wallet_id))
            .filter(Column::Used.eq(false))
            .order_by_desc(Column::CreatedAt)
            .one(&self.db)
            .await
            .map_err(|e| DatabaseError::FindOne(e.to_string()))?;

        Ok(model.map(Into::into))
    }

    async fn find_by_address(&self, address: &str) -> Result<Option<BitcoinAddress>, DatabaseError> {
        let model = Entity::find()
            .filter(Column::Address.eq(address))
            .one(&self.db)
            .await
            .map_err(|e| DatabaseError::FindOne(e.to_string()))?;

        Ok(model.map(Into::into))
    }

    async fn insert(&self, address: BitcoinAddress) -> Result<BitcoinAddress, DatabaseError> {
        let model = ActiveModel {
            id: Set(address.id),
            wallet_id: Set(address.wallet_id),
            address: Set(address.address),
            used: Set(address.used),
            derivation_index: Set(address.derivation_index),
            ..Default::default()
        };

        let model = model
            .insert(&self.db)
            .await
            .map_err(|e| DatabaseError::Insert(e.to_string()))?;

        Ok(model.into())
    }

    async fn mark_used(&self, id: Uuid) -> Result<(), DatabaseError> {
        let active_model = ActiveModel {
            id: Unchanged(id),
            used: Set(true),
            updated_at: Set(Some(Utc::now().naive_utc())),
            ..Default::default()
        };

        active_model
            .update(&self.db)
            .await
            .map_err(|e| DatabaseError::Update(e.to_string()))?;

        Ok(())
    }
}
