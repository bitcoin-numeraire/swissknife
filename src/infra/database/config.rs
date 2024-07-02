use std::time::Duration;

use serde::Deserialize;

use crate::infra::config::config_rs::deserialize_duration;

#[derive(Clone, Debug, Deserialize)]
pub struct DatabaseConfig {
    pub url: String,
    #[serde(deserialize_with = "deserialize_duration")]
    pub connect_timeout: Duration,
    #[serde(deserialize_with = "deserialize_duration")]
    pub idle_timeout: Duration,
    #[serde(deserialize_with = "deserialize_duration")]
    pub acquire_timeout: Duration,
    pub max_connections: Option<u32>,
    pub min_connections: Option<u32>,
    #[serde(deserialize_with = "deserialize_duration")]
    pub max_lifetime: Duration,
    pub sqlx_logging: Option<bool>,
}
