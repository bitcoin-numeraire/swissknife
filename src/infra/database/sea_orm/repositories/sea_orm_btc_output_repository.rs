use async_trait::async_trait;
use chrono::Utc;
use sea_orm::{
    ActiveModelTrait, ColumnTrait, DatabaseConnection, EntityTrait, FromQueryResult, QueryFilter, QuerySelect, Set,
    Unchanged,
};
use uuid::Uuid;

use super::SeaOrmConnection;

use crate::{
    application::errors::DatabaseError,
    domains::bitcoin::{BtcOutput, BtcOutputRepository},
    infra::database::sea_orm::models::{
        btc_output::{ActiveModel, Column},
        prelude::BtcOutput as BtcOutputEntity,
    },
};

#[derive(Clone)]
pub struct SeaOrmBitcoinOutputRepository<C = DatabaseConnection> {
    db: C,
}

impl<C> SeaOrmBitcoinOutputRepository<C> {
    pub fn new(db: C) -> Self {
        Self { db }
    }
}

#[async_trait]
impl<C> BtcOutputRepository for SeaOrmBitcoinOutputRepository<C>
where
    C: SeaOrmConnection,
{
    async fn find_by_outpoint(&self, outpoint: &str) -> Result<Option<BtcOutput>, DatabaseError> {
        let model = BtcOutputEntity::find()
            .filter(Column::Outpoint.eq(outpoint))
            .one(self.db.connection())
            .await
            .map_err(|e| DatabaseError::FindOne(e.to_string()))?;

        Ok(model.map(Into::into))
    }

    async fn upsert(&self, output: BtcOutput) -> Result<BtcOutput, DatabaseError> {
        if let Some(existing) = self.find_by_outpoint(&output.outpoint).await? {
            let active_model = ActiveModel {
                id: Unchanged(existing.id),
                txid: Set(output.txid.clone()),
                output_index: Set(output.output_index as i32),
                address: Set(output.address.clone()),
                amount_sat: Set(output.amount_sat as i64),
                status: Set(output.status.to_string()),
                block_height: Set(output.block_height.map(|h| h as i32)),
                updated_at: Set(Some(Utc::now().naive_utc())),
                ..Default::default()
            };

            let model = active_model
                .update(self.db.connection())
                .await
                .map_err(|e| DatabaseError::Update(e.to_string()))?;

            return Ok(model.into());
        }

        let model = ActiveModel {
            id: Set(Uuid::new_v4()),
            outpoint: Set(output.outpoint.clone()),
            txid: Set(output.txid.clone()),
            output_index: Set(output.output_index as i32),
            address: Set(output.address.clone()),
            amount_sat: Set(output.amount_sat as i64),
            status: Set(output.status.to_string()),
            block_height: Set(output.block_height.map(|h| h as i32)),
            ..Default::default()
        };

        let model = model
            .insert(self.db.connection())
            .await
            .map_err(|e| DatabaseError::Insert(e.to_string()))?;

        Ok(model.into())
    }

    async fn max_block_height(&self) -> Result<Option<u32>, DatabaseError> {
        #[derive(FromQueryResult)]
        struct MaxBlockHeight {
            max: Option<i32>,
        }

        let result = BtcOutputEntity::find()
            .select_only()
            .column_as(Column::BlockHeight.max(), "max")
            .into_model::<MaxBlockHeight>()
            .one(self.db.connection())
            .await
            .map_err(|e| DatabaseError::FindOne(e.to_string()))?;

        Ok(result.and_then(|row| row.max).map(|value| value as u32))
    }
}
