use thiserror::Error;

#[derive(Debug, Error)]
pub enum ConfigError {
    #[error("Failed to load configuration: {0}")]
    Load(String),

    #[error("Missing lightning provider config: {0}")]
    MissingLightningProviderConfig(String),
}
