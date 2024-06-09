use serde::Deserialize;
use std::time::Duration;

use crate::infra::config::config_rs::deserialize_duration;

#[derive(Clone, Debug, Deserialize)]
pub struct AxumServerConfig {
    pub addr: String,
    #[serde(deserialize_with = "deserialize_duration")]
    pub request_timeout: Duration,
}
