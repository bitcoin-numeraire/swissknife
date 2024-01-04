use serde::Deserialize;

use crate::adapters::{
    auth::AuthConfig, database::DatabaseConfig, lightning::breez::BreezClientConfig,
    logging::tracing::TracingLoggerConfig, rgb::rgblib::RGBLibClientConfig,
    web::axum::AxumServerConfig,
};

#[derive(Debug, Deserialize, Clone)]
pub struct AppConfig {
    pub database: DatabaseConfig,
    pub lightning: BreezClientConfig,
    pub rgb: RGBLibClientConfig,
    pub web: AxumServerConfig,
    pub logging: TracingLoggerConfig,
    pub auth: AuthConfig,
}
