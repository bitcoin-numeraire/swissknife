use async_trait::async_trait;
use sea_orm::{ColumnTrait, DatabaseConnection, EntityTrait, QueryFilter};

use super::SeaOrmConnection;

use crate::{
    application::errors::DatabaseError,
    domains::{
        asset::{bitcoin_network_key, Asset, AssetRepository, BITCOIN_PROTOCOL, NATIVE_ASSET_REF},
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
            .filter(Column::Protocol.eq(BITCOIN_PROTOCOL))
            .filter(Column::Network.eq(bitcoin_network_key(network)))
            .filter(Column::AssetRef.eq(NATIVE_ASSET_REF))
            .one(self.db.connection())
            .await
            .map_err(|e| DatabaseError::FindOne(e.to_string()))?;

        Ok(model.map(Into::into))
    }
}
