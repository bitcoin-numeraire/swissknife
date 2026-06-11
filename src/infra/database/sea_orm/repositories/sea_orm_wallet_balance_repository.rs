use async_trait::async_trait;
use chrono::Utc;
use sea_orm::{
    sea_query::{Expr, OnConflict},
    ColumnTrait, DatabaseConnection, EntityTrait, QueryFilter, Set,
};
use uuid::Uuid;

use super::SeaOrmConnection;

use crate::{
    application::{composition::Currency, errors::DatabaseError},
    domains::wallet::WalletBalanceRepository,
    infra::database::sea_orm::models::{
        prelude::WalletBalance,
        wallet_balance::{ActiveModel, Column},
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
    async fn credit(&self, wallet_id: Uuid, currency: &Currency, amount_msat: u64) -> Result<(), DatabaseError> {
        let amount = amount_msat as i64;
        let model = ActiveModel {
            wallet_id: Set(wallet_id),
            currency: Set(currency.to_string()),
            available_amount: Set(amount),
            reserved_amount: Set(0),
            created_at: Set(Utc::now().naive_utc()),
            ..Default::default()
        };

        WalletBalance::insert(model)
            .on_conflict(
                OnConflict::columns([Column::WalletId, Column::Currency])
                    .value(
                        Column::AvailableAmount,
                        Expr::col((WalletBalance, Column::AvailableAmount)).add(amount),
                    )
                    .value(Column::UpdatedAt, Expr::value(Utc::now().naive_utc()))
                    .to_owned(),
            )
            .exec(self.db.connection())
            .await
            .map_err(|e| DatabaseError::Insert(e.to_string()))?;

        Ok(())
    }

    async fn reserve(&self, wallet_id: Uuid, currency: &Currency, amount_msat: u64) -> Result<bool, DatabaseError> {
        let amount = amount_msat as i64;
        let result = WalletBalance::update_many()
            .col_expr(Column::AvailableAmount, Expr::col(Column::AvailableAmount).sub(amount))
            .col_expr(Column::ReservedAmount, Expr::col(Column::ReservedAmount).add(amount))
            .col_expr(Column::UpdatedAt, Expr::value(Utc::now().naive_utc()))
            .filter(Column::WalletId.eq(wallet_id))
            .filter(Column::Currency.eq(currency.to_string()))
            .filter(Column::AvailableAmount.gte(amount))
            .exec(self.db.connection())
            .await
            .map_err(|e| DatabaseError::Update(e.to_string()))?;

        Ok(result.rows_affected == 1)
    }

    async fn debit(&self, wallet_id: Uuid, currency: &Currency, amount_msat: u64) -> Result<bool, DatabaseError> {
        let amount = amount_msat as i64;
        let result = WalletBalance::update_many()
            .col_expr(Column::AvailableAmount, Expr::col(Column::AvailableAmount).sub(amount))
            .col_expr(Column::UpdatedAt, Expr::value(Utc::now().naive_utc()))
            .filter(Column::WalletId.eq(wallet_id))
            .filter(Column::Currency.eq(currency.to_string()))
            .filter(Column::AvailableAmount.gte(amount))
            .exec(self.db.connection())
            .await
            .map_err(|e| DatabaseError::Update(e.to_string()))?;

        Ok(result.rows_affected == 1)
    }

    async fn release(&self, wallet_id: Uuid, currency: &Currency, amount_msat: u64) -> Result<bool, DatabaseError> {
        let amount = amount_msat as i64;
        let result = WalletBalance::update_many()
            .col_expr(Column::AvailableAmount, Expr::col(Column::AvailableAmount).add(amount))
            .col_expr(Column::ReservedAmount, Expr::col(Column::ReservedAmount).sub(amount))
            .col_expr(Column::UpdatedAt, Expr::value(Utc::now().naive_utc()))
            .filter(Column::WalletId.eq(wallet_id))
            .filter(Column::Currency.eq(currency.to_string()))
            .filter(Column::ReservedAmount.gte(amount))
            .exec(self.db.connection())
            .await
            .map_err(|e| DatabaseError::Update(e.to_string()))?;

        Ok(result.rows_affected == 1)
    }
}
