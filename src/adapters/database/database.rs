use serde::Deserialize;
use sqlx::PgPool;

#[derive(Clone, Debug, Deserialize)]
pub struct DatabaseConfig {
    pub url: String,
    pub idle_timeout: Option<String>,
    pub max_connections: Option<u32>,
    pub min_connections: Option<u32>,
    pub max_lifetime: Option<String>,
    pub acquire_timeout: Option<String>,
}

pub trait DatabaseClient: Send + Sync {
    fn pool(&self) -> PgPool;
}
