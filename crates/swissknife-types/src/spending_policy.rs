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
