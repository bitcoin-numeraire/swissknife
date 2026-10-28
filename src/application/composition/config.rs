use std::time::Duration;

use serde::{Deserialize, Deserializer};
use strum_macros::{Display, EnumString};
pub use swissknife_types::AuthProvider;

use crate::{
    domains::bitcoin::BtcAddressType,
    infra::{
        axum::AxumServerConfig,
        config::config_rs::deserialize_duration,
        database::sea_orm::SeaOrmConfig,
        jwt::{local::JwtConfig, oauth2::OAuth2Config},
        lightning::{
            cln::{ClnClientConfig, ClnRestClientConfig},
            ldk::LdkServerConfig,
            lnd::{LndGrpcClientConfig, LndRestClientConfig},
            wavelength::WavelengthConfig,
        },
        logging::tracing::TracingLoggerConfig,
    },
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
    /// A fixed duration, or `provider` to use the daemon's invoice policy.
    #[serde(deserialize_with = "deserialize_invoice_expiry")]
    pub invoice_expiry: Option<Duration>,
    pub client_events: ClientEventConfig,
    #[serde(default)]
    pub bitcoin_address_type: BtcAddressType,
    pub ln_provider: LightningProvider,
    pub database: SeaOrmConfig,
    pub cln_grpc_config: Option<ClnClientConfig>,
    pub cln_rest_config: Option<ClnRestClientConfig>,
    pub lnd_grpc_config: Option<LndGrpcClientConfig>,
    pub lnd_rest_config: Option<LndRestClientConfig>,
    pub ldk_server_config: Option<LdkServerConfig>,
    pub wavelength_config: Option<WavelengthConfig>,
    pub web: AxumServerConfig,
    pub logging: TracingLoggerConfig,
}

#[derive(Debug, Deserialize, Clone)]
pub struct ClientEventConfig {
    #[serde(deserialize_with = "deserialize_duration")]
    pub retention: Duration,
    #[serde(deserialize_with = "deserialize_duration")]
    pub cleanup_interval: Duration,
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

fn deserialize_invoice_expiry<'de, D>(deserializer: D) -> Result<Option<Duration>, D::Error>
where
    D: Deserializer<'de>,
{
    let value = String::deserialize(deserializer)?;
    if value == "provider" {
        return Ok(None);
    }
    let duration = humantime::parse_duration(&value).map_err(serde::de::Error::custom)?;
    if duration.as_secs() == 0 || duration.as_secs() > u64::from(u32::MAX) || duration.subsec_nanos() != 0 {
        return Err(serde::de::Error::custom(
            "invoice_expiry must be positive whole seconds within u32, or provider",
        ));
    }
    Ok(Some(duration))
}

#[derive(Clone, Copy, Debug, Deserialize, EnumString, Display, PartialEq, Eq, Default)]
#[serde(rename_all = "snake_case")]
#[strum(serialize_all = "snake_case")]
pub enum LightningProvider {
    #[default]
    ClnGrpc,
    ClnRest,
    LndGrpc,
    LndRest,
    LdkServer,
    Wavelength,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[derive(Deserialize)]
    struct Expiry {
        #[serde(deserialize_with = "deserialize_invoice_expiry")]
        expiry: Option<Duration>,
    }

    #[test]
    fn fixed_invoice_expiry_preserves_the_configured_duration() {
        let config: Expiry = serde_json::from_value(serde_json::json!({"expiry":"12h"})).unwrap();
        assert_eq!(config.expiry, Some(Duration::from_secs(43_200)));
    }

    #[test]
    fn provider_expiry_is_explicit_and_does_not_invent_a_duration() {
        let config: Expiry = serde_json::from_value(serde_json::json!({"expiry":"provider"})).unwrap();
        assert_eq!(config.expiry, None);
    }

    #[test]
    fn invoice_expiry_rejects_zero_fractional_seconds_and_overflow() {
        for value in ["0s", "1ms", "1.5s", "4294967296s"] {
            assert!(serde_json::from_value::<Expiry>(serde_json::json!({"expiry":value})).is_err());
        }
    }
}
