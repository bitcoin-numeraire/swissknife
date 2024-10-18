use std::time::Duration;

use serde::Deserialize;
use strum_macros::{Display, EnumString};

use crate::infra::{
    axum::AxumServerConfig,
    config::config_rs::deserialize_duration,
    database::sea_orm::SeaOrmConfig,
    jwt::{local::JwtConfig, oauth2::OAuth2Config},
    lightning::{
        breez::BreezClientConfig,
        cln::{ClnClientConfig, ClnRestClientConfig},
        lnd::LndClientConfig,
    },
    logging::tracing::TracingLoggerConfig,
};

#[derive(Debug, Deserialize, Clone)]
pub struct AppConfig {
    pub domain: String,
    pub host: String,
    pub auth_provider: AuthProvider,
    pub oauth2: Option<OAuth2Config>,
    pub jwt: Option<JwtConfig>,
    #[serde(deserialize_with = "deserialize_duration")]
    pub invoice_expiry: Duration,
    pub fee_buffer: Option<f64>,
    pub ln_provider: LightningProvider,
    pub database: SeaOrmConfig,
    pub breez_config: Option<BreezClientConfig>,
    pub cln_grpc_config: Option<ClnClientConfig>,
    pub cln_rest_config: Option<ClnRestClientConfig>,
    pub lnd_config: Option<LndClientConfig>,
    pub web: AxumServerConfig,
    pub logging: TracingLoggerConfig,
}

#[derive(Clone, Debug, Deserialize, EnumString, Display, PartialEq, Eq, Default)]
#[serde(rename_all = "lowercase")]
#[strum(serialize_all = "lowercase")]
pub enum LightningProvider {
    #[default]
    Breez,
    ClnGrpc,
    ClnRest,
    Lnd,
}

#[derive(Clone, Debug, Deserialize, EnumString, Display, PartialEq, Eq, Default)]
#[serde(rename_all = "lowercase")]
#[strum(serialize_all = "lowercase")]
pub enum AuthProvider {
    #[default]
    Jwt,
    OAuth2,
}
