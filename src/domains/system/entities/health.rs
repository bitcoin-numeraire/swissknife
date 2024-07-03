use serde::Serialize;
use strum_macros::{Display, EnumString};

#[derive(Debug, Serialize)]
pub struct HealthCheck {
    pub database: HealthStatus,
    pub ln_provider: HealthStatus,
}

#[derive(Clone, Debug, EnumString, Serialize, Display, PartialEq, Eq)]
pub enum HealthStatus {
    Operational,
    Unavailable,
    Maintenance,
}
