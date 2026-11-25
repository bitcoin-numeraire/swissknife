use super::models::{
    agent_spending_hours as hours,
    prelude::{AgentSpendingHours as HoursEntity, SpendingEntry, SpendingPolicy as PolicyEntity},
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
use swissknife_types::{AgentSpendingHours, Payment, SpendingLimits, SpendingPolicy, SpendingUsage};
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
            agent_hourly_msat: row.agent_hourly_msat.map(|n| n as u64),
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

impl From<hours::Model> for AgentSpendingHours {
    fn from(row: hours::Model) -> Self {
        Self {
            weekdays: (1..=7).filter(|day| row.weekdays & (1 << (day - 1)) != 0).collect(),
            start_minute: row.start_minute as u16,
            end_minute: row.end_minute as u16,
        }
    }
}

#[async_trait]
impl SpendingPolicyRepository for SeaOrmSpendingPolicyRepository {
    async fn hours(&self, account: Uuid) -> Result<Option<AgentSpendingHours>, DatabaseError> {
        Ok(HoursEntity::find_by_id(account)
            .one(&self.0)
            .await
            .map_err(db_error)?
            .map(Into::into))
    }

    async fn set_hours(&self, account: Uuid, hours: Option<AgentSpendingHours>) -> Result<(), DatabaseError> {
        let Some(window) = hours else {
            HoursEntity::delete_by_id(account)
                .exec(&self.0)
                .await
                .map_err(db_error)?;
            return Ok(());
        };
        use hours::Column::*;
        let weekdays = window
            .weekdays
            .into_iter()
            .fold(0i16, |mask, day| mask | (1 << (day - 1)));
        HoursEntity::insert(hours::ActiveModel {
            account_id: Set(account),
            weekdays: Set(weekdays),
            start_minute: Set(window.start_minute as i16),
            end_minute: Set(window.end_minute as i16),
        })
        .on_conflict(
            OnConflict::column(AccountId)
                .update_columns([Weekdays, StartMinute, EndMinute])
                .to_owned(),
        )
        .exec(&self.0)
        .await
        .map_err(db_error)?;
        Ok(())
    }

    async fn report(
        &self,
        account: Uuid,
        range: crate::domains::agent::SpendingReportRange,
    ) -> Result<Vec<swissknife_types::SpendingReportRow>, DatabaseError> {
        use futures_util::TryStreamExt;
        use std::collections::BTreeMap;
        use swissknife_types::{SpendingInterval, SpendingReportRow};
        let mut query = SpendingEntry::find()
            .select_only()
            .column(entry::Column::AdmittedAt)
            .column(entry::Column::ChargedMsat)
            .filter(entry::Column::AccountId.eq(account))
            .filter(entry::Column::AdmittedAt.gte(range.start))
            .filter(entry::Column::AdmittedAt.lt(range.end));
        if let Some(id) = range.wallet_id {
            query = query.filter(entry::Column::WalletId.eq(id));
        }
        if let Some(id) = range.agent_key_id {
            query = query.filter(entry::Column::AgentKeyId.eq(id));
        }
        let selector = query.into_tuple::<(chrono::NaiveDateTime, i64)>();
        let mut entries = selector.stream(&self.0).await.map_err(db_error)?;
        // A long report can exceed both a database BIGINT and JavaScript's exact number range.
        // Keep at most 366 groups in memory and sum original integer amounts.
        let mut groups = BTreeMap::<NaiveDate, (u64, u128)>::new();
        while let Some((admission, charge)) = entries.try_next().await.map_err(db_error)? {
            let day = admission.date();
            let start = match range.interval {
                SpendingInterval::Day => day,
                SpendingInterval::Week => period_starts(day)[1],
            };
            let group = groups.entry(start).or_default();
            group.0 += 1;
            group.1 += u128::try_from(charge)
                .map_err(|_| DatabaseError::FindMany("Negative account spending charge".into()))?;
        }
        Ok(groups
            .into_iter()
            .map(|(period_start, (payments, total))| SpendingReportRow {
                period_start,
                payments,
                charged_msat: total.to_string(),
            })
            .collect())
    }

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
            agent_hourly_msat: Set(limits.agent_hourly_msat.map(|n| n as i64)),
        })
        .on_conflict(
            OnConflict::column(AccountId)
                .update_columns([
                    Enabled,
                    PerPaymentMsat,
                    DailyMsat,
                    WeeklyMsat,
                    MonthlyMsat,
                    AgentHourlyMsat,
                ])
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

pub(super) async fn admit(
    tx: &DatabaseTransaction,
    payment: &Payment,
    amount: u64,
    agent_key_id: Option<Uuid>,
) -> Result<(), ApplicationError> {
    let account = wallet::Entity::find_by_id(payment.wallet_id)
        .select_only()
        .column(wallet::Column::AccountId)
        .into_tuple::<Uuid>()
        .one(tx)
        .await
        .map_err(db_error)?
        .ok_or_else(|| DataError::NotFound("Wallet not found".into()))?;
    let now = Utc::now();
    if agent_key_id.is_some() {
        if let Some(hours) = HoursEntity::find_by_id(account)
            .lock_shared()
            .one(tx)
            .await
            .map_err(db_error)?
        {
            crate::domains::agent::authorize_hours(&hours.into(), now)?;
        }
    }
    let Some(row) = locked_policy(tx, account).await?.filter(|row| row.enabled) else {
        return Ok(());
    };
    let limits: SpendingLimits = row.into();
    if let (Some(key), Some(cap)) = (agent_key_id, limits.agent_hourly_msat) {
        authorize_agent_hour(tx, account, key, now, cap, amount).await?;
    }
    let used = usage(tx, account, now.date_naive()).await?;
    authorize_spending(&limits, &used, amount)?;
    entry::ActiveModel {
        payment_id: Set(payment.id),
        wallet_id: Set(Some(payment.wallet_id)),
        agent_key_id: Set(agent_key_id),
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

async fn authorize_agent_hour(
    tx: &DatabaseTransaction,
    account: Uuid,
    key: Uuid,
    now: chrono::DateTime<Utc>,
    cap: u64,
    amount: u64,
) -> Result<(), ApplicationError> {
    use futures_util::TryStreamExt;
    let exceeded = || {
        DataError::Validation(format!(
            "Agent rolling hourly spending limit of {cap} msat exceeded (including fees)"
        ))
    };
    let mut remaining = cap.checked_sub(amount).ok_or_else(exceeded)?;
    // The caller holds the account policy lock throughout admission. Subtract
    // exact charges incrementally so a historical total can exceed SQL BIGINT.
    let entries = SpendingEntry::find()
        .select_only()
        .column(entry::Column::ChargedMsat)
        .filter(entry::Column::AccountId.eq(account))
        .filter(entry::Column::AgentKeyId.eq(key))
        .filter(entry::Column::AdmittedAt.gt((now - chrono::Duration::hours(1)).naive_utc()))
        .into_tuple::<i64>();
    let mut rows = entries.stream(tx).await.map_err(db_error)?;
    while let Some(value) = rows.try_next().await.map_err(db_error)? {
        let charge =
            u64::try_from(value).map_err(|_| DataError::Inconsistency("Negative agent spending entry".into()))?;
        remaining = remaining.checked_sub(charge).ok_or_else(exceeded)?;
    }
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
