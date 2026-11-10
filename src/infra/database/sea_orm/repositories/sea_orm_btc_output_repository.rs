use async_trait::async_trait;
use chrono::Utc;
use sea_orm::{
    sea_query::{Expr, OnConflict},
    ColumnTrait, DatabaseConnection, EntityTrait, ExprTrait, FromQueryResult, QueryFilter, QuerySelect, Set,
};
use uuid::Uuid;

use super::SeaOrmConnection;

use crate::{
    application::errors::DatabaseError,
    domains::bitcoin::{BtcOutput, BtcOutputRepository, BtcOutputStatus},
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

    async fn upsert(&self, output: BtcOutput) -> Result<Option<BtcOutput>, DatabaseError> {
        let mut conflict = OnConflict::column(Column::Outpoint);
        conflict
            .update_columns([Column::Address, Column::AmountSat, Column::Status, Column::BlockHeight])
            .value(Column::UpdatedAt, Expr::value(Utc::now().naive_utc()));
        if output.status == BtcOutputStatus::Unconfirmed {
            // A mempool observation can reach this transaction after CLN
            // already processed the block containing the same outpoint.
            let height = Expr::col((BtcOutputEntity, Column::BlockHeight));
            conflict.action_and_where(height.clone().is_null().or(height.eq(0)));
        }

        let affected = BtcOutputEntity::insert(ActiveModel {
            id: Set(Uuid::new_v4()),
            outpoint: Set(output.outpoint.clone()),
            txid: Set(output.txid),
            output_index: Set(i64::from(output.output_index)),
            address: Set(output.address),
            amount_sat: Set(output.amount_sat as i64),
            status: Set(output.status.to_string()),
            block_height: Set(output.block_height.map(i64::from)),
            ..Default::default()
        })
        .on_conflict(conflict.to_owned())
        .exec_without_returning(self.db.connection())
        .await
        .map_err(|e| DatabaseError::Insert(e.to_string()))?;

        if affected == 0 {
            return Ok(None);
        }
        let output = self
            .find_by_outpoint(&output.outpoint)
            .await?
            .ok_or_else(|| DatabaseError::FindOne("Bitcoin output missing after upsert".into()))?;
        Ok(Some(output))
    }

    async fn max_block_height(&self) -> Result<Option<u32>, DatabaseError> {
        #[derive(FromQueryResult)]
        struct MaxBlockHeight {
            max: Option<i64>,
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
