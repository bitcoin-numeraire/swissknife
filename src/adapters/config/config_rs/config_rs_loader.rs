#[cfg(debug_assertions)]
use dotenv::dotenv;

use std::env;

use config::{Config, Environment, File};

use crate::{
    adapters::config::ConfigLoader,
    application::{dtos::AppConfig, errors::ConfigError},
};

const ENV_PREFIX: &str = "swissknife";
const DEFAULT_RUN_MODE: &str = "development";
const CONFIG_PATH: &str = "config";
const DEFAULT_FILENAME: &str = "default";

pub struct ConfigRsLoader {}

impl ConfigLoader for ConfigRsLoader {
    fn load(&self) -> Result<AppConfig, ConfigError> {
        // Load .env file in development
        #[cfg(debug_assertions)]
        dotenv().ok();

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
            .map_err(|e| ConfigError::Load(e.to_string()))?;

        settings
            .try_deserialize()
            .map_err(|e| ConfigError::Load(e.to_string()))
    }
}
