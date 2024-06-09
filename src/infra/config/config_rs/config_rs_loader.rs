use std::{env, time::Duration};

use config::{Config, Environment, File};
use humantime::parse_duration;
use serde::{Deserialize, Deserializer};

use crate::application::{dtos::AppConfig, errors::ConfigError};

const ENV_PREFIX: &str = "swissknife";
const DEFAULT_RUN_MODE: &str = "development";
const CONFIG_PATH: &str = "config";
const DEFAULT_FILENAME: &str = "default";

pub fn load_config() -> Result<AppConfig, ConfigError> {
    let run_mode = env::var("RUN_MODE").unwrap_or_else(|_| DEFAULT_RUN_MODE.into());

    let settings = Config::builder()
        .add_source(
            File::with_name(&format!("{}/{}", CONFIG_PATH, DEFAULT_FILENAME)).required(false),
        )
        .add_source(File::with_name(&format!("{}/{}", CONFIG_PATH, run_mode)).required(false))
        .add_source(
            Environment::with_prefix(ENV_PREFIX)
                .prefix_separator("_")
                .separator("__"),
        )
        .build()
        .map_err(|e| ConfigError::Load(e.to_string()))?
        .try_deserialize()
        .map_err(|e| ConfigError::Load(e.to_string()))?;

    Ok(settings)
}

pub fn deserialize_duration<'de, D>(deserializer: D) -> Result<Duration, D::Error>
where
    D: Deserializer<'de>,
{
    let s = String::deserialize(deserializer)?;
    parse_duration(&s).map_err(serde::de::Error::custom)
}
