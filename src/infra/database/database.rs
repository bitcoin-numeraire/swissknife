use std::future::Future;

use async_trait::async_trait;
use serde::Deserialize;
use sqlx::{PgPool, Postgres, Transaction};

use crate::application::errors::DatabaseError;

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

#[async_trait]
pub trait DatabaseClient: Send + Sync {}

#[async_trait]
pub trait TransactionManager: Send + Sync {
    async fn run_in_transaction<F, T>(&self, func: F) -> Result<T, DatabaseError>
    where
        F: FnOnce(&mut Transaction<'_, Postgres>) -> T + Send + 'static,
        T: Future<Output = Result<T, DatabaseError>> + Send;
}
