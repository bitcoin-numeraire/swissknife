use sea_orm::{ConnectOptions, Database, DatabaseConnection};
use tracing::{debug, trace};

use crate::{application::errors::DatabaseError, infra::database::DatabaseConfig};
use migration::{Migrator, MigratorTrait};

#[derive(Clone)]
pub struct SeaORMClient {}

impl SeaORMClient {
    pub async fn connect(config: DatabaseConfig) -> Result<DatabaseConnection, DatabaseError> {
        let mut opt = ConnectOptions::new(config.url);

        opt.connect_timeout(config.connect_timeout);
        opt.idle_timeout(config.idle_timeout);
        opt.acquire_timeout(config.acquire_timeout);
        opt.max_lifetime(config.max_lifetime);

        if let Some(max_connections) = config.max_connections {
            opt.max_connections(max_connections);
        }

        if let Some(min_connections) = config.min_connections {
            opt.min_connections(min_connections);
        }

        if let Some(sqlx_logging) = config.sqlx_logging {
            opt.sqlx_logging(sqlx_logging);
        }

        let db_conn = Database::connect(opt)
            .await
            .map_err(|e| DatabaseError::Connect(e.to_string()))?;

        trace!("Executing migrations...");
        Migrator::up(&db_conn, None)
            .await
            .map_err(|e| DatabaseError::Migrations(e.to_string()))?;
        debug!("Migrations executed successfully");

        Ok(db_conn)
    }
}
