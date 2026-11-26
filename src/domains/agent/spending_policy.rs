use super::ReviewedPayment;
use crate::application::errors::{ApplicationError, DataError, DatabaseError};
use crate::domains::payment::SubmissionKey;
use async_trait::async_trait;
use chrono::{Datelike, Days, NaiveDate};
use std::sync::Arc;
use swissknife_types::{
    AgentSpendingHours, SpendingInterval, SpendingLimits, SpendingPolicy, SpendingReportQuery, SpendingReportRow,
    SpendingUsage,
};
use swissknife_types::{ApprovalDecision, PaymentApproval};
use uuid::Uuid;

#[cfg_attr(test, mockall::automock)]
#[async_trait]
pub trait SpendingPolicyRepository: Send + Sync {
    async fn activity(
        &self,
        account: Uuid,
        as_of: chrono::DateTime<chrono::Utc>,
        after: Option<Uuid>,
        limit: u32,
    ) -> Result<swissknife_types::AgentActivityPage, DatabaseError>;
    async fn reviewed(&self, key: &SubmissionKey) -> Result<Option<ReviewedPayment>, ApplicationError>;
    async fn pending_approvals(&self, account: Uuid) -> Result<Vec<PaymentApproval>, ApplicationError>;
    async fn approval(&self, account: Uuid, id: Uuid) -> Result<PaymentApproval, ApplicationError>;
    async fn decide(
        &self,
        account: Uuid,
        id: Uuid,
        operator: Uuid,
        decision: ApprovalDecision,
    ) -> Result<PaymentApproval, ApplicationError>;

    async fn hours(&self, account: Uuid) -> Result<Option<AgentSpendingHours>, DatabaseError>;
    async fn set_hours(&self, account: Uuid, hours: Option<AgentSpendingHours>) -> Result<(), DatabaseError>;
    async fn report(&self, account: Uuid, range: SpendingReportRange) -> Result<Vec<SpendingReportRow>, DatabaseError>;
    async fn get(&self, account: Uuid) -> Result<Option<SpendingPolicy>, DatabaseError>;
    async fn replace(&self, account: Uuid, limits: SpendingLimits) -> Result<(), DatabaseError>;
    async fn disable(&self, account: Uuid) -> Result<(), DatabaseError>;
}

#[cfg_attr(test, mockall::automock)]
#[async_trait]
pub trait SpendingPolicyUseCases: Send + Sync {
    async fn activity(
        &self,
        account: Uuid,
        query: swissknife_types::AgentActivityQuery,
    ) -> Result<swissknife_types::AgentActivityPage, ApplicationError>;
    async fn pending_approvals(&self, account: Uuid) -> Result<Vec<PaymentApproval>, ApplicationError>;
    async fn approval(&self, account: Uuid, id: Uuid) -> Result<PaymentApproval, ApplicationError>;
    async fn decide(
        &self,
        account: Uuid,
        id: Uuid,
        operator: Uuid,
        decision: ApprovalDecision,
    ) -> Result<PaymentApproval, ApplicationError>;

    async fn hours(&self, account: Uuid) -> Result<AgentSpendingHours, ApplicationError>;
    async fn set_hours(&self, account: Uuid, hours: Option<AgentSpendingHours>) -> Result<(), ApplicationError>;
    async fn report(
        &self,
        account: Uuid,
        query: SpendingReportQuery,
    ) -> Result<Vec<SpendingReportRow>, ApplicationError>;
    async fn get(&self, account: Uuid) -> Result<SpendingPolicy, ApplicationError>;
    async fn replace(&self, account: Uuid, limits: SpendingLimits) -> Result<(), ApplicationError>;
    async fn disable(&self, account: Uuid) -> Result<(), ApplicationError>;
}

pub struct SpendingReportRange {
    pub start: chrono::NaiveDateTime,
    pub end: chrono::NaiveDateTime,
    pub wallet_id: Option<Uuid>,
    pub agent_key_id: Option<Uuid>,
    pub interval: SpendingInterval,
}

pub struct SpendingPolicyService(pub Arc<dyn SpendingPolicyRepository>);

#[async_trait]
impl SpendingPolicyUseCases for SpendingPolicyService {
    async fn activity(
        &self,
        account: Uuid,
        query: swissknife_types::AgentActivityQuery,
    ) -> Result<swissknife_types::AgentActivityPage, ApplicationError> {
        let limit = query.limit.unwrap_or(25);
        if !(1..=100).contains(&limit) {
            return Err(DataError::Validation("Request between 1 and 100 agent credentials per page".into()).into());
        }
        Ok(self.0.activity(account, chrono::Utc::now(), query.after, limit).await?)
    }

    async fn pending_approvals(&self, account: Uuid) -> Result<Vec<PaymentApproval>, ApplicationError> {
        self.0.pending_approvals(account).await
    }
    async fn approval(&self, account: Uuid, id: Uuid) -> Result<PaymentApproval, ApplicationError> {
        self.0.approval(account, id).await
    }
    async fn decide(
        &self,
        account: Uuid,
        id: Uuid,
        operator: Uuid,
        decision: ApprovalDecision,
    ) -> Result<PaymentApproval, ApplicationError> {
        self.0.decide(account, id, operator, decision).await
    }

    async fn hours(&self, account: Uuid) -> Result<AgentSpendingHours, ApplicationError> {
        self.0
            .hours(account)
            .await?
            .ok_or_else(|| DataError::NotFound("No delegated spending hours configured".into()).into())
    }
    async fn set_hours(&self, account: Uuid, mut hours: Option<AgentSpendingHours>) -> Result<(), ApplicationError> {
        if let Some(ref mut window) = hours {
            super::spending_hours::validate_hours(window)?;
        }
        Ok(self.0.set_hours(account, hours).await?)
    }
    async fn report(
        &self,
        account: Uuid,
        query: SpendingReportQuery,
    ) -> Result<Vec<SpendingReportRow>, ApplicationError> {
        let last = query.through.unwrap_or_else(|| chrono::Utc::now().date_naive());
        let first = query
            .from
            .or_else(|| last.checked_sub_days(Days::new(29)))
            .ok_or_else(|| DataError::Validation("Invalid spending report start date".into()))?;
        let end = last
            .succ_opt()
            .ok_or_else(|| DataError::Validation("Invalid spending report end date".into()))?;
        if first > last || (end - first).num_days() > 366 {
            return Err(DataError::Validation("Choose a spending report range of 1–366 days".into()).into());
        }
        let range = SpendingReportRange {
            start: first.and_time(chrono::NaiveTime::MIN),
            end: end.and_time(chrono::NaiveTime::MIN),
            wallet_id: query.wallet_id,
            agent_key_id: query.agent_key_id,
            interval: query.interval,
        };
        Ok(self.0.report(account, range).await?)
    }

    async fn get(&self, account: Uuid) -> Result<SpendingPolicy, ApplicationError> {
        self.0
            .get(account)
            .await?
            .ok_or_else(|| DataError::NotFound("No active spending policy".into()).into())
    }

    async fn replace(&self, account: Uuid, limits: SpendingLimits) -> Result<(), ApplicationError> {
        let amounts = [
            limits.per_payment_msat,
            limits.daily_msat,
            limits.weekly_msat,
            limits.monthly_msat,
            limits.agent_hourly_msat,
            limits.agent_hourly_warning_msat,
            limits.approval_above_msat,
        ];
        if amounts.iter().all(Option::is_none) {
            return Err(DataError::Validation("Set at least one spending limit".into()).into());
        }
        if amounts.into_iter().flatten().any(|amount| amount > i64::MAX as u64) {
            return Err(DataError::Validation("Spending limit exceeds the ledger amount range".into()).into());
        }
        Ok(self.0.replace(account, limits).await?)
    }

    async fn disable(&self, account: Uuid) -> Result<(), ApplicationError> {
        Ok(self.0.disable(account).await?)
    }
}

/// Calendar periods containing the instant when the ledger first admits the payment.
pub(crate) fn period_starts(day: NaiveDate) -> [NaiveDate; 3] {
    [
        day,
        day - Days::new(day.weekday().num_days_from_monday().into()),
        day.with_day(1).expect("every month has a first day"),
    ]
}

pub(crate) fn authorize_spending(limits: &SpendingLimits, usage: &SpendingUsage, amount: u64) -> Result<(), DataError> {
    for (name, cap, charged) in [
        ("per-payment", limits.per_payment_msat, 0),
        ("daily", limits.daily_msat, usage.daily_msat),
        ("weekly", limits.weekly_msat, usage.weekly_msat),
        ("monthly", limits.monthly_msat, usage.monthly_msat),
    ] {
        if let Some(cap) = cap {
            if u128::from(amount) > u128::from(cap).saturating_sub(charged) {
                return Err(DataError::Validation(format!(
                    "Account {name} spending limit of {cap} msat exceeded (including fees)"
                )));
            }
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn calendar_periods_and_limits_are_exact_at_year_and_month_boundaries() {
        let date = |year, month, day| NaiveDate::from_ymd_opt(year, month, day).unwrap();
        assert_eq!(
            period_starts(date(2027, 1, 1)),
            [date(2027, 1, 1), date(2026, 12, 28), date(2027, 1, 1)]
        );
        assert_eq!(
            period_starts(date(2026, 11, 30)),
            [date(2026, 11, 30), date(2026, 11, 30), date(2026, 11, 1)]
        );
        let usage = SpendingUsage {
            daily_msat: 10,
            weekly_msat: 20,
            monthly_msat: 30,
        };
        let limits = SpendingLimits {
            per_payment_msat: Some(4),
            daily_msat: Some(14),
            weekly_msat: Some(24),
            monthly_msat: Some(34),
            ..Default::default()
        };
        assert!(authorize_spending(&limits, &usage, 4).is_ok());
        for limits in [
            SpendingLimits {
                per_payment_msat: Some(3),
                ..Default::default()
            },
            SpendingLimits {
                daily_msat: Some(13),
                ..Default::default()
            },
            SpendingLimits {
                weekly_msat: Some(23),
                ..Default::default()
            },
            SpendingLimits {
                monthly_msat: Some(33),
                ..Default::default()
            },
            SpendingLimits {
                monthly_msat: Some(0),
                ..Default::default()
            },
        ] {
            assert!(authorize_spending(&limits, &usage, 4).is_err());
        }
    }
}
