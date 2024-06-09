use std::time::Duration;

use serde::Deserialize;
use strum_macros::{Display, EnumString};

use crate::infra::{
    auth::AuthConfig,
    axum::AxumServerConfig,
    config::config_rs::deserialize_duration,
    database::DatabaseConfig,
    lightning::{
        breez::BreezClientConfig,
        cln::{ClnClientConfig, ClnRestClientConfig},
    },
    logging::tracing::TracingLoggerConfig,
};

#[derive(Debug, Deserialize, Clone)]
pub struct AppConfig {
    pub domain: String,
    #[serde(deserialize_with = "deserialize_duration")]
    pub invoice_expiry: Duration,
    pub fee_buffer: Option<f64>,
    pub ln_provider: LightningProvider,
    pub database: DatabaseConfig,
    pub breez_config: Option<BreezClientConfig>,
    pub cln_config: Option<ClnClientConfig>,
    pub cln_rest_config: Option<ClnRestClientConfig>,
    pub web: AxumServerConfig,
    pub logging: TracingLoggerConfig,
    pub auth: AuthConfig,
}

#[derive(Clone, Debug, Deserialize, EnumString, Display, PartialEq, Eq, Default)]
#[serde(rename_all = "lowercase")]
#[strum(serialize_all = "lowercase")]
pub enum LightningProvider {
    #[default]
    Breez,
    Cln,
    ClnRest,
}
