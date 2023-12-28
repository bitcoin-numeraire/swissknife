use serde::Deserialize;
use sqlx::PgPool;
use std::sync::Arc;

use async_trait::async_trait;

#[derive(Clone, Debug, Deserialize)]
pub struct DatabaseConfig {
    pub url: String,
    pub idle_timeout: Option<String>,
    pub max_connections: Option<u32>,
    pub min_connections: Option<u32>,
    pub max_lifetime: Option<String>,
    pub acquire_timeout: Option<String>,
}

#[async_trait]
pub trait DatabaseClient {
    fn pool(&self) -> PgPool;
}
pub type DynDatabaseClient = Arc<dyn DatabaseClient + Send + Sync>;
