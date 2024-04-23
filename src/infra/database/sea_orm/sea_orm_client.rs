use std::time::Duration;

use humantime::parse_duration;
use sea_orm::{ConnectOptions, Database, DatabaseConnection};

use crate::{application::errors::DatabaseError, infra::database::DatabaseConfig};

#[derive(Clone)]
pub struct SeaORMClient {}

impl SeaORMClient {
    pub async fn connect(config: DatabaseConfig) -> Result<DatabaseConnection, DatabaseError> {
        let mut opt = ConnectOptions::new(config.url);

        if let Some(connect_timeout_str) = config.connect_timeout {
            let connect_timeout = Self::parse_duration(&connect_timeout_str)?;
            opt.connect_timeout(connect_timeout);
        }

        if let Some(idle_timeout_str) = config.idle_timeout {
            let idle_timeout = Self::parse_duration(&idle_timeout_str)?;
            opt.idle_timeout(idle_timeout);
        }

        if let Some(acquire_timeout_str) = config.acquire_timeout {
            let acquire_timeout = Self::parse_duration(&acquire_timeout_str)?;
            opt.acquire_timeout(acquire_timeout);
        }

        if let Some(max_connections) = config.max_connections {
            opt.max_connections(max_connections);
        }

        if let Some(min_connections) = config.min_connections {
            opt.min_connections(min_connections);
        }

        if let Some(max_lifetime_str) = config.max_lifetime {
            let max_lifetime = Self::parse_duration(&max_lifetime_str)?;
            opt.max_lifetime(max_lifetime);
        }

        if let Some(sqlx_logging) = config.sqlx_logging {
            opt.sqlx_logging(sqlx_logging);
        }

        let db_conn = Database::connect(opt)
            .await
            .map_err(|e| DatabaseError::Connect(e.to_string()))?;

        Ok(db_conn)
    }

    fn parse_duration(duration_str: &str) -> Result<Duration, DatabaseError> {
        parse_duration(duration_str).map_err(|e| DatabaseError::ParseConfig(e.to_string()))
    }
}
