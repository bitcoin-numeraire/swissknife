use async_trait::async_trait;
use sea_orm::{ColumnTrait, DatabaseConnection, EntityTrait, QueryFilter};

use super::SeaOrmConnection;

use crate::{
    application::errors::DatabaseError,
    domains::{
        asset::{Asset, AssetRepository, Protocol, NATIVE_ASSET_REF},
        bitcoin::BtcNetwork,
    },
    infra::database::sea_orm::models::{asset::Column, prelude::Asset as AssetEntity},
};

#[derive(Clone)]
pub struct SeaOrmAssetRepository<C = DatabaseConnection> {
    db: C,
}

impl<C> SeaOrmAssetRepository<C> {
    pub fn new(db: C) -> Self {
        Self { db }
    }
}

#[async_trait]
impl<C> AssetRepository for SeaOrmAssetRepository<C>
where
    C: SeaOrmConnection,
{
    async fn find_native_btc_by_network(&self, network: BtcNetwork) -> Result<Option<Asset>, DatabaseError> {
        let model = AssetEntity::find()
            .filter(Column::Protocol.eq(Protocol::Bitcoin.to_string()))
            .filter(Column::Network.eq(network.to_string()))
            .filter(Column::AssetRef.eq(NATIVE_ASSET_REF))
            .one(self.db.connection())
            .await
            .map_err(|e| DatabaseError::FindOne(e.to_string()))?;

        Ok(model.map(Into::into))
    }
}
