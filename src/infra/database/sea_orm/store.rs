use std::sync::Arc;

use async_trait::async_trait;
use migration::{Migrator, MigratorTrait};
use sea_orm::{ConnectOptions, Database, DatabaseConnection};
use tracing::{debug, trace};

use crate::{
    application::{entities::AppStore, errors::DatabaseError},
    domains::system::HealthProbe,
};

use super::{
    SeaOrmApiKeyRepository, SeaOrmBitcoinAddressRepository, SeaOrmBitcoinOutputRepository, SeaOrmConfig,
    SeaOrmConfigRepository, SeaOrmEventProjectionUnitOfWork, SeaOrmInvoiceRepository, SeaOrmLnAddressRepository,
    SeaOrmPaymentRepository, SeaOrmPaymentUnitOfWork, SeaOrmWalletRepository,
};

pub struct SeaOrmStore;

impl SeaOrmStore {
    pub async fn connect(config: SeaOrmConfig) -> Result<AppStore, DatabaseError> {
        let db_conn = Self::connect_database(config).await?;
        Ok(Self::from_connection(db_conn))
    }

    pub fn from_connection(db_conn: DatabaseConnection) -> AppStore {
        AppStore::new(
            Arc::new(SeaOrmLnAddressRepository::new(db_conn.clone())),
            Arc::new(SeaOrmPaymentRepository::new(db_conn.clone())),
            Arc::new(SeaOrmInvoiceRepository::new(db_conn.clone())),
            Arc::new(SeaOrmWalletRepository::new(db_conn.clone())),
            Arc::new(SeaOrmApiKeyRepository::new(db_conn.clone())),
            Arc::new(SeaOrmConfigRepository::new(db_conn.clone())),
            Arc::new(SeaOrmBitcoinAddressRepository::new(db_conn.clone())),
            Arc::new(SeaOrmBitcoinOutputRepository::new(db_conn.clone())),
            Arc::new(SeaOrmHealthProbe::new(db_conn.clone())),
            Arc::new(SeaOrmPaymentUnitOfWork::new(db_conn.clone())),
            Arc::new(SeaOrmEventProjectionUnitOfWork::new(db_conn)),
        )
    }

    async fn connect_database(config: SeaOrmConfig) -> Result<DatabaseConnection, DatabaseError> {
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

        trace!("Running database migrations");
        Migrator::up(&db_conn, None)
            .await
            .map_err(|e| DatabaseError::Migrations(e.to_string()))?;
        debug!("Database migrations completed successfully");

        Ok(db_conn)
    }
}

#[derive(Clone)]
pub struct SeaOrmHealthProbe {
    db: DatabaseConnection,
}

impl SeaOrmHealthProbe {
    pub fn new(db: DatabaseConnection) -> Self {
        Self { db }
    }
}

#[async_trait]
impl HealthProbe for SeaOrmHealthProbe {
    async fn ping(&self) -> Result<(), DatabaseError> {
        self.db.ping().await.map_err(|e| DatabaseError::Ping(e.to_string()))
    }
}
