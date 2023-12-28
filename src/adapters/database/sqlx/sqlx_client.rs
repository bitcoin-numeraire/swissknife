use async_trait::async_trait;
use humantime::parse_duration;
use sqlx::{postgres::PgPoolOptions, PgPool};

use crate::{
    adapters::database::{DatabaseClient, DatabaseConfig},
    application::errors::DatabaseError,
};

pub struct SQLxClient {
    pool: PgPool,
}

impl SQLxClient {
    pub async fn connect(config: DatabaseConfig) -> Result<Self, DatabaseError> {
        let mut pool_options = PgPoolOptions::new();

        if let Some(idle_timeout_str) = config.idle_timeout {
            let idle_timeout = parse_duration(&idle_timeout_str)
                .map_err(|e| DatabaseError::Connect(e.to_string()))?;

            pool_options = pool_options.idle_timeout(idle_timeout);
        }

        if let Some(max_connections) = config.max_connections {
            pool_options = pool_options.max_connections(max_connections);
        }

        if let Some(min_connections) = config.min_connections {
            pool_options = pool_options.min_connections(min_connections);
        }

        if let Some(max_lifetime_str) = config.max_lifetime {
            let max_lifetime = parse_duration(&max_lifetime_str)
                .map_err(|e| DatabaseError::Connect(e.to_string()))?;

            pool_options = pool_options.max_lifetime(max_lifetime);
        }

        let pool = pool_options
            .connect(&config.url)
            .await
            .map_err(|e| DatabaseError::Connect(e.to_string()))?;

        Ok(Self { pool })
    }
}

#[async_trait]
impl DatabaseClient for SQLxClient {}
