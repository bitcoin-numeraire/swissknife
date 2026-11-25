use super::models::{
    prelude::{SpendingEntry, SpendingPolicy as PolicyEntity},
    spending_entry as entry, spending_policy as policy, wallet,
};
use crate::{
    application::errors::{ApplicationError, DataError, DatabaseError},
    domains::agent::{authorize_spending, period_starts, SpendingPolicyRepository},
};
use async_trait::async_trait;
use chrono::{NaiveDate, Utc};
use sea_orm::{
    sea_query::{Alias, Expr, OnConflict},
    ActiveModelTrait, ColumnTrait, ConnectionTrait, DatabaseConnection, DatabaseTransaction, EntityTrait, ExprTrait,
    QueryFilter, QuerySelect, Set,
};
use swissknife_types::{Payment, SpendingLimits, SpendingPolicy, SpendingUsage};
use uuid::Uuid;

pub struct SeaOrmSpendingPolicyRepository(pub DatabaseConnection);

fn db_error(err: sea_orm::DbErr) -> DatabaseError {
    DatabaseError::Transaction(err.to_string())
}

impl From<policy::Model> for SpendingLimits {
    fn from(row: policy::Model) -> Self {
        Self {
            per_payment_msat: row.per_payment_msat.map(|n| n as u64),
            daily_msat: row.daily_msat.map(|n| n as u64),
            weekly_msat: row.weekly_msat.map(|n| n as u64),
            monthly_msat: row.monthly_msat.map(|n| n as u64),
        }
    }
}

async fn usage(db: &impl ConnectionTrait, account: Uuid, today: NaiveDate) -> Result<SpendingUsage, DatabaseError> {
    let mut totals = [0; 3];
    for (total, date) in totals.iter_mut().zip(period_starts(today)) {
        let amount = SpendingEntry::find()
            .select_only()
            .column_as(entry::Column::ChargedMsat.sum().cast_as(Alias::new("BIGINT")), "amount")
            .filter(entry::Column::AccountId.eq(account))
            .filter(entry::Column::AdmittedAt.gte(date.and_time(chrono::NaiveTime::MIN)))
            .into_tuple::<Option<i64>>()
            .one(db)
            .await
            .map_err(db_error)?
            .flatten()
            .unwrap_or(0);
        *total = u64::try_from(amount).map_err(|_| DatabaseError::FindOne("Negative account spending usage".into()))?;
    }
    let [daily_msat, weekly_msat, monthly_msat] = totals;
    Ok(SpendingUsage {
        daily_msat,
        weekly_msat,
        monthly_msat,
    })
}

#[async_trait]
impl SpendingPolicyRepository for SeaOrmSpendingPolicyRepository {
    async fn get(&self, account: Uuid) -> Result<Option<SpendingPolicy>, DatabaseError> {
        use sea_orm::TransactionTrait;
        let tx = self.0.begin().await.map_err(db_error)?;
        let row = PolicyEntity::find_by_id(account)
            .filter(policy::Column::Enabled.eq(true))
            .lock_shared()
            .one(&tx)
            .await
            .map_err(db_error)?;
        let result = match row {
            Some(row) => Some(SpendingPolicy {
                limits: row.into(),
                usage: usage(&tx, account, Utc::now().date_naive()).await?,
            }),
            None => None,
        };
        tx.commit().await.map_err(db_error)?;
        Ok(result)
    }

    async fn replace(&self, account: Uuid, limits: SpendingLimits) -> Result<(), DatabaseError> {
        use policy::Column::*;
        // The service validates the signed ledger range before this conversion.
        PolicyEntity::insert(policy::ActiveModel {
            account_id: Set(account),
            enabled: Set(true),
            per_payment_msat: Set(limits.per_payment_msat.map(|n| n as i64)),
            daily_msat: Set(limits.daily_msat.map(|n| n as i64)),
            weekly_msat: Set(limits.weekly_msat.map(|n| n as i64)),
            monthly_msat: Set(limits.monthly_msat.map(|n| n as i64)),
        })
        .on_conflict(
            OnConflict::column(AccountId)
                .update_columns([Enabled, PerPaymentMsat, DailyMsat, WeeklyMsat, MonthlyMsat])
                .to_owned(),
        )
        .exec(&self.0)
        .await
        .map_err(db_error)?;
        Ok(())
    }

    async fn disable(&self, account: Uuid) -> Result<(), DatabaseError> {
        PolicyEntity::update_many()
            .col_expr(policy::Column::Enabled, Expr::value(false))
            .filter(policy::Column::AccountId.eq(account))
            .exec(&self.0)
            .await
            .map_err(db_error)?;
        Ok(())
    }
}

// Admission and settlement already wrote the wallet or payment in this transaction,
// acquiring SQLite's writer lock. PostgreSQL additionally locks the shared account row.
async fn locked_policy(tx: &DatabaseTransaction, account: Uuid) -> Result<Option<policy::Model>, DatabaseError> {
    PolicyEntity::find_by_id(account)
        .lock_exclusive()
        .one(tx)
        .await
        .map_err(db_error)
}

pub(super) async fn admit(tx: &DatabaseTransaction, payment: &Payment, amount: u64) -> Result<(), ApplicationError> {
    let account = wallet::Entity::find_by_id(payment.wallet_id)
        .select_only()
        .column(wallet::Column::AccountId)
        .into_tuple::<Uuid>()
        .one(tx)
        .await
        .map_err(db_error)?
        .ok_or_else(|| DataError::NotFound("Wallet not found".into()))?;
    let Some(row) = locked_policy(tx, account).await?.filter(|row| row.enabled) else {
        return Ok(());
    };
    let now = Utc::now();
    let limits = row.into();
    let used = usage(tx, account, now.date_naive()).await?;
    authorize_spending(&limits, &used, amount)?;
    entry::ActiveModel {
        payment_id: Set(payment.id),
        account_id: Set(account),
        admitted_at: Set(now.naive_utc()),
        charged_msat: Set(i64::try_from(amount)
            .map_err(|_| DataError::Validation("Payment exceeds the ledger amount range".into()))?),
    }
    .insert(tx)
    .await
    .map_err(db_error)?;
    Ok(())
}

pub(super) async fn adjust(tx: &DatabaseTransaction, payment: Uuid, amount: u64) -> Result<(), ApplicationError> {
    let Some(existing) = SpendingEntry::find_by_id(payment).one(tx).await.map_err(db_error)? else {
        return Ok(());
    };
    // A disabled policy still tracks the outcome of its outstanding reservations.
    locked_policy(tx, existing.account_id).await?;
    SpendingEntry::update_many()
        .col_expr(
            entry::Column::ChargedMsat,
            Expr::value(
                i64::try_from(amount)
                    .map_err(|_| DataError::Inconsistency("Settled amount exceeds ledger range".into()))?,
            ),
        )
        .filter(entry::Column::PaymentId.eq(payment))
        .exec(tx)
        .await
        .map_err(db_error)?;
    Ok(())
}
