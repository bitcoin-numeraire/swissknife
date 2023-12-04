use serde::Deserialize;

use crate::adapters::{
    lightning::breez::BreezClientConfig, logging::tracing::TracingLoggerConfig,
    rgb::rgblib::RGBLibClientConfig, web::axum::AxumServerConfig,
};

#[derive(Debug, Deserialize)]
pub struct AppConfig {
    pub lightning: BreezClientConfig,
    pub rgb: RGBLibClientConfig,
    pub web: AxumServerConfig,
    pub logging: TracingLoggerConfig,
}
