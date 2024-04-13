use async_trait::async_trait;
use serde::Deserialize;
use sqlx::{Database, Pool};

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
pub trait DatabaseClient: Send + Sync {
    type DB: Database;

    fn pool(&self) -> &Pool<Self::DB>;
}
