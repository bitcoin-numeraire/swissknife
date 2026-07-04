use async_trait::async_trait;
use chrono::Utc;
use sea_orm::{sea_query::Expr, ColumnTrait, DatabaseConnection, EntityTrait, QueryFilter};
use uuid::Uuid;

use super::SeaOrmConnection;

use crate::{
    application::{composition::Currency, errors::DatabaseError},
    domains::wallet::WalletBalanceRepository,
    infra::database::sea_orm::models::{
        prelude::Wallet,
        wallet::{Column, Entity as WalletEntity},
    },
};

#[derive(Clone)]
pub struct SeaOrmWalletBalanceRepository<C = DatabaseConnection> {
    db: C,
}

impl<C> SeaOrmWalletBalanceRepository<C> {
    pub fn new(db: C) -> Self {
        Self { db }
    }
}

#[async_trait]
impl<C> WalletBalanceRepository for SeaOrmWalletBalanceRepository<C>
where
    C: SeaOrmConnection,
{
    async fn credit(&self, wallet_id: Uuid, _currency: &Currency, amount_msat: u64) -> Result<(), DatabaseError> {
        let amount = amount_msat as i64;
        let result = WalletEntity::update_many()
            .col_expr(Column::AvailableAmount, Expr::col(Column::AvailableAmount).add(amount))
            .col_expr(Column::UpdatedAt, Expr::value(Utc::now().naive_utc()))
            .filter(Column::Id.eq(wallet_id))
            .exec(self.db.connection())
            .await
            .map_err(|e| DatabaseError::Update(e.to_string()))?;

        if result.rows_affected == 1 {
            Ok(())
        } else {
            Err(DatabaseError::Update(
                "wallet balance credit target was missing".to_string(),
            ))
        }
    }

    async fn reserve(&self, wallet_id: Uuid, _currency: &Currency, amount_msat: u64) -> Result<bool, DatabaseError> {
        let amount = amount_msat as i64;
        let result = Wallet::update_many()
            .col_expr(Column::AvailableAmount, Expr::col(Column::AvailableAmount).sub(amount))
            .col_expr(Column::ReservedAmount, Expr::col(Column::ReservedAmount).add(amount))
            .col_expr(Column::UpdatedAt, Expr::value(Utc::now().naive_utc()))
            .filter(Column::Id.eq(wallet_id))
            .filter(Column::AvailableAmount.gte(amount))
            .exec(self.db.connection())
            .await
            .map_err(|e| DatabaseError::Update(e.to_string()))?;

        Ok(result.rows_affected == 1)
    }

    async fn debit(&self, wallet_id: Uuid, _currency: &Currency, amount_msat: u64) -> Result<bool, DatabaseError> {
        let amount = amount_msat as i64;
        let result = Wallet::update_many()
            .col_expr(Column::AvailableAmount, Expr::col(Column::AvailableAmount).sub(amount))
            .col_expr(Column::UpdatedAt, Expr::value(Utc::now().naive_utc()))
            .filter(Column::Id.eq(wallet_id))
            .filter(Column::AvailableAmount.gte(amount))
            .exec(self.db.connection())
            .await
            .map_err(|e| DatabaseError::Update(e.to_string()))?;

        Ok(result.rows_affected == 1)
    }

    async fn debit_confirmed(
        &self,
        wallet_id: Uuid,
        _currency: &Currency,
        amount_msat: u64,
    ) -> Result<bool, DatabaseError> {
        let amount = amount_msat as i64;
        let result = Wallet::update_many()
            .col_expr(Column::AvailableAmount, Expr::col(Column::AvailableAmount).sub(amount))
            .col_expr(Column::UpdatedAt, Expr::value(Utc::now().naive_utc()))
            .filter(Column::Id.eq(wallet_id))
            .exec(self.db.connection())
            .await
            .map_err(|e| DatabaseError::Update(e.to_string()))?;

        Ok(result.rows_affected == 1)
    }

    async fn release(&self, wallet_id: Uuid, _currency: &Currency, amount_msat: u64) -> Result<bool, DatabaseError> {
        let amount = amount_msat as i64;
        let result = Wallet::update_many()
            .col_expr(Column::AvailableAmount, Expr::col(Column::AvailableAmount).add(amount))
            .col_expr(Column::ReservedAmount, Expr::col(Column::ReservedAmount).sub(amount))
            .col_expr(Column::UpdatedAt, Expr::value(Utc::now().naive_utc()))
            .filter(Column::Id.eq(wallet_id))
            .filter(Column::ReservedAmount.gte(amount))
            .exec(self.db.connection())
            .await
            .map_err(|e| DatabaseError::Update(e.to_string()))?;

        Ok(result.rows_affected == 1)
    }
}
