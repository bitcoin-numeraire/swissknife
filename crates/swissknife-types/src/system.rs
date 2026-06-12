use serde::{Deserialize, Serialize};
use strum_macros::{Display, EnumString};
use utoipa::ToSchema;

/// App health information, fine-grained by dependency.
#[derive(Debug, Deserialize, Serialize, ToSchema)]
pub struct HealthCheck {
    /// Health of the database
    pub database: HealthStatus,
    /// Health of the Lightning provider service
    pub ln_provider: HealthStatus,
    /// Whether the system is healthy and can be used
    pub is_healthy: bool,
}

/// Health of a single system dependency.
#[derive(Clone, Debug, EnumString, Deserialize, Serialize, Display, PartialEq, Eq, ToSchema)]
pub enum HealthStatus {
    Operational,
    Unavailable,
    Maintenance,
}

impl HealthCheck {
    pub fn new(database: HealthStatus, ln_provider: HealthStatus) -> Self {
        let is_healthy = database == HealthStatus::Operational && ln_provider == HealthStatus::Operational;
        Self {
            database,
            ln_provider,
            is_healthy,
        }
    }
}

/// App setup info.
#[derive(Debug, Deserialize, Serialize, ToSchema)]
pub struct SetupInfo {
    /// Whether the welcome flow has been completed
    pub welcome_complete: bool,
    /// Whether the admin user has been created
    pub sign_up_complete: bool,
}

/// App version info.
#[derive(Debug, Deserialize, Serialize, ToSchema)]
pub struct VersionInfo {
    /// Current version of the software
    #[schema(example = "0.0.1")]
    pub version: String,

    /// Build time of the software
    #[schema(example = "2024-07-03T18:13:09.093289+00:00")]
    pub build_time: String,
}
