use serde::{Deserialize, Serialize};
use serde_with::{serde_as, DisplayFromStr};
use utoipa::ToSchema;

/// Account-wide limits, including fees. Amounts are decimal millisatoshi strings.
/// Omitted limits are unrestricted. Zero blocks positive payments.
#[serde_as]
#[derive(Clone, Debug, Default, Deserialize, Serialize, ToSchema)]
#[serde(deny_unknown_fields)]
pub struct SpendingLimits {
    #[serde_as(as = "Option<DisplayFromStr>")]
    #[schema(value_type = Option<String>, example = "50000")]
    pub per_payment_msat: Option<u64>,
    #[serde_as(as = "Option<DisplayFromStr>")]
    #[schema(value_type = Option<String>)]
    pub daily_msat: Option<u64>,
    #[serde_as(as = "Option<DisplayFromStr>")]
    #[schema(value_type = Option<String>)]
    pub weekly_msat: Option<u64>,
    #[serde_as(as = "Option<DisplayFromStr>")]
    #[schema(value_type = Option<String>)]
    pub monthly_msat: Option<u64>,
}

/// Consumption in the current UTC day, ISO week and calendar month.
#[serde_as]
#[derive(Clone, Debug, Default, Deserialize, Serialize, ToSchema)]
pub struct SpendingUsage {
    #[serde_as(as = "DisplayFromStr")]
    #[schema(value_type = String)]
    pub daily_msat: u64,
    #[serde_as(as = "DisplayFromStr")]
    #[schema(value_type = String)]
    pub weekly_msat: u64,
    #[serde_as(as = "DisplayFromStr")]
    #[schema(value_type = String)]
    pub monthly_msat: u64,
}

#[derive(Clone, Debug, Deserialize, Serialize, ToSchema)]
pub struct SpendingPolicy {
    pub limits: SpendingLimits,
    pub usage: SpendingUsage,
}

#[derive(Clone, Copy, Debug, Default, Deserialize, Serialize, ToSchema)]
#[serde(rename_all = "lowercase")]
pub enum SpendingInterval {
    #[default]
    Day,
    Week,
}

#[derive(Default, Deserialize, utoipa::IntoParams)]
pub struct SpendingReportQuery {
    /// First UTC admission date, inclusive. Defaults to 29 days before the last date.
    #[param(value_type = Option<String>, example = "2026-11-01")]
    pub from: Option<chrono::NaiveDate>,
    /// Last UTC admission date, inclusive. Defaults to today.
    #[param(value_type = Option<String>, example = "2026-11-30")]
    pub through: Option<chrono::NaiveDate>,
    pub wallet_id: Option<uuid::Uuid>,
    /// Filter by the credential that originally admitted a payment, including revoked keys.
    pub agent_key_id: Option<uuid::Uuid>,
    #[serde(default)]
    pub interval: SpendingInterval,
}

#[derive(Debug, Deserialize, Serialize, ToSchema)]
pub struct SpendingReportRow {
    /// UTC day or Monday of the ISO week.
    #[schema(value_type = String, example = "2026-11-01")]
    pub period_start: chrono::NaiveDate,
    /// All admitted payments in the selected period, including subsequently released reservations.
    pub payments: u64,
    /// Current amount charged, in exact decimal millisatoshis, including outstanding reservations.
    pub charged_msat: String,
}

/// UTC window for new payments submitted with a delegated wallet credential.
#[derive(Clone, Debug, Deserialize, Serialize, ToSchema, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct AgentSpendingHours {
    /// ISO weekdays on which the window starts (1 = Monday, 7 = Sunday).
    pub weekdays: Vec<u8>,
    /// Included UTC minute of day, from 0 to 1439.
    pub start_minute: u16,
    /// Excluded UTC minute of day, from 0 to 1440. An earlier end crosses midnight.
    pub end_minute: u16,
}
