use serde::Deserialize;
use strum_macros::{Display, EnumString};

use crate::infra::{
    auth::AuthConfig,
    axum::AxumServerConfig,
    database::DatabaseConfig,
    lightning::{breez::BreezClientConfig, cln::ClnClientConfig},
    logging::tracing::TracingLoggerConfig,
};

#[derive(Debug, Deserialize, Clone)]
pub struct AppConfig {
    pub domain: String,
    pub invoice_expiry: Option<u32>,
    pub fee_buffer: Option<f64>,
    pub lightning_provider: LightningProvider,
    pub database: DatabaseConfig,
    pub breez_config: Option<BreezClientConfig>,
    pub cln_config: Option<ClnClientConfig>,
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
}
