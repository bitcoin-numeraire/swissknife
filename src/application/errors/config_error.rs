use thiserror::Error;

#[derive(Debug, Error)]
pub enum ConfigError {
    #[error("Set SWISSKNIFE_JWT__SECRET to a deployment-specific random secret for local authentication")]
    InvalidLocalJwtSecret,

    #[error("Failed to load configuration: {0}")]
    Load(String),

    #[error("Missing lightning provider config: {0}")]
    MissingLightningProviderConfig(String),

    #[error("Missing auth provider config: {0}")]
    MissingAuthProviderConfig(String),
}
