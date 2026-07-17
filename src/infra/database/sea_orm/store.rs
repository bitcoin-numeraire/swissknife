use std::{str::FromStr, sync::Arc};

use async_trait::async_trait;
use migration::{Migrator, MigratorTrait};
use sea_orm::{
    sqlx::{
        sqlite::{SqliteConnectOptions, SqliteJournalMode, SqlitePoolOptions, SqliteSynchronous},
        ConnectOptions as SqlxConnectOptions,
    },
    ConnectOptions, Database, DatabaseConnection, SqlxSqliteConnector,
};
use tracing::{debug, trace};

use crate::{
    application::{composition::AppStore, errors::DatabaseError},
    domains::system::HealthProbe,
};

use super::{
    SeaOrmAccountRepository, SeaOrmApiKeyRepository, SeaOrmAssetRepository, SeaOrmBitcoinAddressRepository,
    SeaOrmBitcoinOutputRepository, SeaOrmClientEventRepository, SeaOrmConfig, SeaOrmConfigRepository,
    SeaOrmEventProjectionUnitOfWork, SeaOrmInvoiceRepository, SeaOrmLnAddressRepository, SeaOrmPaymentRepository,
    SeaOrmPaymentUnitOfWork, SeaOrmWalletRepository,
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
            Arc::new(SeaOrmAccountRepository::new(db_conn.clone())),
            Arc::new(SeaOrmAssetRepository::new(db_conn.clone())),
            Arc::new(SeaOrmApiKeyRepository::new(db_conn.clone())),
            Arc::new(SeaOrmConfigRepository::new(db_conn.clone())),
            Arc::new(SeaOrmBitcoinAddressRepository::new(db_conn.clone())),
            Arc::new(SeaOrmBitcoinOutputRepository::new(db_conn.clone())),
            Arc::new(SeaOrmClientEventRepository::new(db_conn.clone())),
            Arc::new(SeaOrmHealthProbe::new(db_conn.clone())),
            Arc::new(SeaOrmPaymentUnitOfWork::new(db_conn.clone())),
            Arc::new(SeaOrmEventProjectionUnitOfWork::new(db_conn)),
        )
    }

    async fn connect_database(config: SeaOrmConfig) -> Result<DatabaseConnection, DatabaseError> {
        // SQLite needs WAL + a busy_timeout to survive concurrent writers.
        // sea-orm's ConnectOptions exposes no SQLite pragma API and sqlx-sqlite ignores
        // them as URL query params, so build the sqlx pool directly for SQLite. Postgres
        // serializes writes and keeps the standard sea-orm path.
        let db_conn = if config.url.starts_with("sqlite:") {
            Self::connect_sqlite(&config).await?
        } else {
            Self::connect_generic(config).await?
        };

        trace!("Running database migrations");
        Migrator::up(&db_conn, None)
            .await
            .map_err(|e| DatabaseError::Migrations(e.to_string()))?;
        debug!("Database migrations completed successfully");

        Ok(db_conn)
    }

    async fn connect_generic(config: SeaOrmConfig) -> Result<DatabaseConnection, DatabaseError> {
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

        Database::connect(opt)
            .await
            .map_err(|e| DatabaseError::Connect(e.to_string()))
    }

    async fn connect_sqlite(config: &SeaOrmConfig) -> Result<DatabaseConnection, DatabaseError> {
        let mut connect_opts = SqliteConnectOptions::from_str(&config.url)
            .map_err(|e| DatabaseError::Connect(e.to_string()))?
            // WAL lets readers proceed without blocking the single writer; the busy
            // timeout makes a contending writer wait for the lock instead of failing
            // instantly with SQLITE_BUSY. FULL keeps commits durable (a crash cannot
            // lose a committed deposit). Writers stay serialized, so balance updates
            // (atomic `SET x = x ± n` statements) are unaffected.
            .journal_mode(SqliteJournalMode::Wal)
            .busy_timeout(config.busy_timeout)
            .synchronous(SqliteSynchronous::Full);

        if !config.sqlx_logging.unwrap_or(false) {
            connect_opts = connect_opts.disable_statement_logging();
        }

        let mut pool_opts = SqlitePoolOptions::new()
            .acquire_timeout(config.acquire_timeout)
            .idle_timeout(config.idle_timeout)
            .max_lifetime(config.max_lifetime);

        if let Some(max_connections) = config.max_connections {
            pool_opts = pool_opts.max_connections(max_connections);
        }

        if let Some(min_connections) = config.min_connections {
            pool_opts = pool_opts.min_connections(min_connections);
        }

        let pool = pool_opts
            .connect_with(connect_opts)
            .await
            .map_err(|e| DatabaseError::Connect(e.to_string()))?;

        debug!(busy_timeout = ?config.busy_timeout, "SQLite connection pool configured with WAL journal mode");

        Ok(SqlxSqliteConnector::from_sqlx_sqlite_pool(pool))
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
