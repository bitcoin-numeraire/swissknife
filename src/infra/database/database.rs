use serde::Deserialize;

#[derive(Clone, Debug, Deserialize)]
pub struct DatabaseConfig {
    pub url: String,
    pub connect_timeout: Option<String>,
    pub idle_timeout: Option<String>,
    pub acquire_timeout: Option<String>,
    pub max_connections: Option<u32>,
    pub min_connections: Option<u32>,
    pub max_lifetime: Option<String>,
    pub sqlx_logging: Option<bool>,
}
