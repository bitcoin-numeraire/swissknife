use serde::Deserialize;
use std::sync::Arc;

use async_trait::async_trait;

#[derive(Clone, Debug, Deserialize)]
pub struct DatabaseConfig {
    pub url: String,
    pub idle_timeout: Option<String>,
    pub max_connections: Option<u32>,
    pub min_connections: Option<u32>,
    pub max_lifetime: Option<String>,
}

#[async_trait]
pub trait DatabaseClient {}
pub type DynDatabaseClient = Arc<dyn DatabaseClient + Send + Sync>;
