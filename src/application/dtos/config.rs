use serde::Deserialize;

use crate::infra::{
    auth::AuthConfig, axum::AxumServerConfig, database::DatabaseConfig,
    lightning::breez::BreezClientConfig, logging::tracing::TracingLoggerConfig,
};

#[derive(Debug, Deserialize, Clone)]
pub struct AppConfig {
    pub database: DatabaseConfig,
    pub lightning: BreezClientConfig,
    pub web: AxumServerConfig,
    pub logging: TracingLoggerConfig,
    pub auth: AuthConfig,
}
