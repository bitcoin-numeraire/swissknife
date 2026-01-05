use async_trait::async_trait;
use chrono::Utc;
use sea_orm::{ActiveModelTrait, ColumnTrait, DatabaseConnection, EntityTrait, QueryFilter, Set, Unchanged};
use uuid::Uuid;

use crate::{
    application::errors::DatabaseError,
    domains::bitcoin::{BitcoinOutput, BitcoinOutputRepository},
    infra::database::sea_orm::models::btc_output::{ActiveModel, Column, Entity},
};

#[derive(Clone)]
pub struct SeaOrmBitcoinOutputRepository {
    pub db: DatabaseConnection,
}

impl SeaOrmBitcoinOutputRepository {
    pub fn new(db: DatabaseConnection) -> Self {
        Self { db }
    }
}

#[async_trait]
impl BitcoinOutputRepository for SeaOrmBitcoinOutputRepository {
    async fn find_by_outpoint(&self, outpoint: &str) -> Result<Option<BitcoinOutput>, DatabaseError> {
        let model = Entity::find()
            .filter(Column::Outpoint.eq(outpoint))
            .one(&self.db)
            .await
            .map_err(|e| DatabaseError::FindOne(e.to_string()))?;

        Ok(model.map(Into::into))
    }

    async fn find(&self, id: Uuid) -> Result<Option<BitcoinOutput>, DatabaseError> {
        let model = Entity::find_by_id(id)
            .one(&self.db)
            .await
            .map_err(|e| DatabaseError::FindOne(e.to_string()))?;

        Ok(model.map(Into::into))
    }

    async fn upsert(&self, output: BitcoinOutput) -> Result<BitcoinOutput, DatabaseError> {
        if let Some(existing) = self.find_by_outpoint(&output.outpoint).await? {
            let active_model = ActiveModel {
                id: Unchanged(existing.id),
                txid: Set(output.txid.clone()),
                output_index: Set(output.output_index as i32),
                amount_sat: Set(output.amount_sat),
                fee_sat: Set(output.fee_sat),
                block_height: Set(output.block_height.map(|h| h as i32)),
                timestamp: Set(output.timestamp.map(|t| t.naive_utc())),
                currency: Set(output.currency.to_string()),
                updated_at: Set(Some(Utc::now().naive_utc())),
                ..Default::default()
            };

            let model = active_model
                .update(&self.db)
                .await
                .map_err(|e| DatabaseError::Update(e.to_string()))?;

            return Ok(model.into());
        }

        let model = ActiveModel {
            id: Set(output.id),
            outpoint: Set(output.outpoint.clone()),
            txid: Set(output.txid.clone()),
            output_index: Set(output.output_index as i32),
            amount_sat: Set(output.amount_sat),
            fee_sat: Set(output.fee_sat),
            block_height: Set(output.block_height.map(|h| h as i32)),
            timestamp: Set(output.timestamp.map(|t| t.naive_utc())),
            currency: Set(output.currency.to_string()),
            ..Default::default()
        };

        let model = model
            .insert(&self.db)
            .await
            .map_err(|e| DatabaseError::Insert(e.to_string()))?;

        Ok(model.into())
    }
}
