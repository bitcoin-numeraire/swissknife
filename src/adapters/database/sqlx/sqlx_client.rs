use std::time::Duration;

use async_trait::async_trait;
use humantime::parse_duration;
use sqlx::{postgres::PgPoolOptions, PgPool};

use crate::{
    adapters::database::{DatabaseClient, DatabaseConfig},
    application::errors::DatabaseError,
};

#[derive(Clone)]
pub struct SQLxClient {
    pool: PgPool,
}

impl SQLxClient {
    pub async fn connect(config: DatabaseConfig) -> Result<Self, DatabaseError> {
        let mut pool_options = PgPoolOptions::new();

        if let Some(idle_timeout_str) = config.idle_timeout {
            let idle_timeout = Self::parse_duration(&idle_timeout_str)?;
            pool_options = pool_options.idle_timeout(idle_timeout);
        }

        if let Some(max_connections) = config.max_connections {
            pool_options = pool_options.max_connections(max_connections);
        }

        if let Some(min_connections) = config.min_connections {
            pool_options = pool_options.min_connections(min_connections);
        }

        if let Some(max_lifetime_str) = config.max_lifetime {
            let max_lifetime = Self::parse_duration(&max_lifetime_str)?;
            pool_options = pool_options.max_lifetime(max_lifetime);
        }

        if let Some(acquire_timeout_str) = config.acquire_timeout {
            let acquire_timeout = Self::parse_duration(&acquire_timeout_str)?;
            pool_options = pool_options.acquire_timeout(acquire_timeout);
        }

        let pool = pool_options
            .connect(&config.url)
            .await
            .map_err(|e| DatabaseError::Connect(e.to_string()))?;

        Ok(Self { pool })
    }

    fn parse_duration(duration_str: &str) -> Result<Duration, DatabaseError> {
        parse_duration(duration_str).map_err(|e| DatabaseError::ParseConfig(e.to_string()))
    }
}

#[async_trait]
impl DatabaseClient for SQLxClient {
    fn pool(&self) -> PgPool {
        self.pool.clone()
    }
}
