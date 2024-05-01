use serde::Deserialize;

use crate::infra::{
    auth::AuthConfig, axum::AxumServerConfig, database::DatabaseConfig, lightning::LightningConfig,
    logging::tracing::TracingLoggerConfig,
};

#[derive(Debug, Deserialize, Clone)]
pub struct AppConfig {
    pub database: DatabaseConfig,
    pub lightning: LightningConfig,
    pub web: AxumServerConfig,
    pub logging: TracingLoggerConfig,
    pub auth: AuthConfig,
}
