use async_trait::async_trait;
use chrono::Utc;
use sea_orm::{
    ActiveModelTrait, ColumnTrait, DatabaseConnection, EntityTrait, QueryFilter, QueryOrder, QuerySelect, QueryTrait,
    Set, Unchanged,
};
use uuid::Uuid;

use crate::{
    application::errors::DatabaseError,
    domains::bitcoin::{BtcAddress, BtcAddressFilter, BtcAddressRepository, BtcAddressType},
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
impl BtcAddressRepository for SeaOrmBitcoinAddressRepository {
    async fn find(&self, id: Uuid) -> Result<Option<BtcAddress>, DatabaseError> {
        let model = Entity::find_by_id(id)
            .one(&self.db)
            .await
            .map_err(|e| DatabaseError::FindOne(e.to_string()))?;

        Ok(model.map(Into::into))
    }

    async fn find_by_wallet_unused(
        &self,
        wallet_id: Uuid,
        address_type: BtcAddressType,
    ) -> Result<Option<BtcAddress>, DatabaseError> {
        let model = Entity::find()
            .filter(Column::WalletId.eq(wallet_id))
            .filter(Column::Used.eq(false))
            .filter(Column::AddressType.eq(address_type.to_string()))
            .order_by_desc(Column::CreatedAt)
            .one(&self.db)
            .await
            .map_err(|e| DatabaseError::FindOne(e.to_string()))?;

        Ok(model.map(Into::into))
    }

    async fn find_by_address(&self, address: &str) -> Result<Option<BtcAddress>, DatabaseError> {
        let model = Entity::find()
            .filter(Column::Address.eq(address))
            .one(&self.db)
            .await
            .map_err(|e| DatabaseError::FindOne(e.to_string()))?;

        Ok(model.map(Into::into))
    }

    async fn find_many(&self, filter: BtcAddressFilter) -> Result<Vec<BtcAddress>, DatabaseError> {
        let models = Entity::find()
            .apply_if(filter.wallet_id, |q, id| q.filter(Column::WalletId.eq(id)))
            .apply_if(filter.address, |q, address| q.filter(Column::Address.eq(address)))
            .apply_if(filter.address_type, |q, address_type| q.filter(Column::AddressType.eq(address_type.to_string())))
            .apply_if(filter.ids, |q, ids| q.filter(Column::Id.is_in(ids)))
            .apply_if(filter.used, |q, active| q.filter(Column::Used.eq(active)))
            .order_by(Column::CreatedAt, filter.order_direction.into())
            .offset(filter.offset)
            .limit(filter.limit)
            .all(&self.db)
            .await
            .map_err(|e| DatabaseError::FindMany(e.to_string()))?;

        Ok(models.into_iter().map(Into::into).collect())
    }

    async fn insert(
        &self,
        wallet_id: Uuid,
        address: &str,
        address_type: BtcAddressType
    ) -> Result<BtcAddress, DatabaseError> {
        let model = ActiveModel {
            id: Set(Uuid::new_v4()),
            wallet_id: Set(wallet_id),
            address: Set(address.to_owned()),
            address_type: Set(address_type.to_string()),
            used: Set(false),
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

    async fn delete_many(&self, filter: BtcAddressFilter) -> Result<u64, DatabaseError> {
        let result = Entity::delete_many()
            .apply_if(filter.wallet_id, |q, id| q.filter(Column::WalletId.eq(id)))
            .apply_if(filter.ids, |q, ids| q.filter(Column::Id.is_in(ids)))
            .apply_if(filter.address, |q, address| q.filter(Column::Address.eq(address)))
            .exec(&self.db)
            .await
            .map_err(|e| DatabaseError::Delete(e.to_string()))?;

        Ok(result.rows_affected)
    }
}
