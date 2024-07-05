use serde::Serialize;
use strum_macros::{Display, EnumString};
use utoipa::ToSchema;

/// App Health Information
#[derive(Debug, Serialize, ToSchema)]
pub struct HealthCheck {
    /// Health of the database
    pub database: HealthStatus,
    /// Health of the Lightning provider service
    pub ln_provider: HealthStatus,
}

#[derive(Clone, Debug, EnumString, Serialize, Display, PartialEq, Eq, ToSchema)]
pub enum HealthStatus {
    Operational,
    Unavailable,
    Maintenance,
}
