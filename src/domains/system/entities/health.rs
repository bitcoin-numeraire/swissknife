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
    /// Whether the system is healthy and can be used
    pub is_healthy: bool,
}

#[derive(Clone, Debug, EnumString, Serialize, Display, PartialEq, Eq, ToSchema)]
pub enum HealthStatus {
    Operational,
    Unavailable,
    Maintenance,
}

impl HealthCheck {
    pub fn new(database: HealthStatus, ln_provider: HealthStatus) -> Self {
        let is_healthy =
            database == HealthStatus::Operational && ln_provider == HealthStatus::Operational;
        Self {
            database,
            ln_provider,
            is_healthy,
        }
    }
}
