use std::time::Duration;

use serde::{Deserialize, Deserializer};
use strum_macros::{Display, EnumString};

use crate::{
    domains::bitcoin::BtcAddressType, infra::{
        axum::AxumServerConfig,
        config::config_rs::deserialize_duration,
        database::sea_orm::SeaOrmConfig,
        jwt::{local::JwtConfig, oauth2::OAuth2Config},
        lightning::{
            breez::BreezClientConfig,
            cln::{ClnClientConfig, ClnRestClientConfig},
            lnd::LndRestClientConfig,
        },
        logging::tracing::TracingLoggerConfig,
    }
};

#[derive(Debug, Deserialize, Clone)]
pub struct AppConfig {
    #[serde(default, deserialize_with = "deserialize_optional_string")]
    pub dashboard_dir: Option<String>,
    pub domain: String,
    pub host: String,
    pub auth_provider: AuthProvider,
    pub oauth2: Option<OAuth2Config>,
    pub jwt: Option<JwtConfig>,
    #[serde(deserialize_with = "deserialize_duration")]
    pub invoice_expiry: Duration,
    pub fee_buffer: Option<f64>,
    #[serde(default)]
    pub bitcoin_address_type: BtcAddressType,
    pub ln_provider: LightningProvider,
    pub database: SeaOrmConfig,
    pub breez_config: Option<BreezClientConfig>,
    pub cln_grpc_config: Option<ClnClientConfig>,
    pub cln_rest_config: Option<ClnRestClientConfig>,
    pub lnd_config: Option<LndRestClientConfig>,
    pub web: AxumServerConfig,
    pub logging: TracingLoggerConfig,
}

fn deserialize_optional_string<'de, D>(deserializer: D) -> Result<Option<String>, D::Error>
where
    D: Deserializer<'de>,
{
    let value = Option::<String>::deserialize(deserializer)?;

    Ok(value.and_then(|s| {
        let trimmed = s.trim();
        if trimmed.is_empty() {
            None
        } else {
            Some(trimmed.to_string())
        }
    }))
}

#[derive(Clone, Copy, Debug, Deserialize, EnumString, Display, PartialEq, Eq, Default)]
#[serde(rename_all = "lowercase")]
#[strum(serialize_all = "lowercase")]
pub enum LightningProvider {
    #[default]
    Breez,
    ClnGrpc,
    ClnRest,
    Lnd,
}

#[derive(Clone, Copy, Debug, Deserialize, EnumString, Display, PartialEq, Eq, Default)]
#[serde(rename_all = "lowercase")]
#[strum(serialize_all = "lowercase")]
pub enum AuthProvider {
    #[default]
    Jwt,
    OAuth2,
}
