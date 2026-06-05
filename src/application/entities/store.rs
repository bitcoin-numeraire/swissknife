use std::sync::Arc;

use sea_orm::{DatabaseConnection, DatabaseTransaction, TransactionTrait};

use crate::{
    application::errors::DatabaseError,
    domains::{
        bitcoin::{BtcAddressRepository, BtcOutputRepository},
        invoice::InvoiceRepository,
        ln_address::LnAddressRepository,
        payment::PaymentRepository,
        system::ConfigRepository,
        user::ApiKeyRepository,
        wallet::WalletRepository,
    },
    infra::database::sea_orm::{
        SeaOrmApiKeyRepository, SeaOrmBitcoinAddressRepository, SeaOrmBitcoinOutputRepository, SeaOrmConfigRepository,
        SeaOrmInvoiceRepository, SeaOrmLnAddressRepository, SeaOrmPaymentRepository, SeaOrmWalletRepository,
    },
};

#[cfg(test)]
use crate::domains::{
    bitcoin::{MockBtcAddressRepository, MockBtcOutputRepository},
    invoice::MockInvoiceRepository,
    ln_address::MockLnAddressRepository,
    payment::MockPaymentRepository,
    system::MockConfigRepository,
    user::MockApiKeyRepository,
    wallet::MockWalletRepository,
};

#[derive(Clone)]
pub struct AppStore {
    db_conn: DatabaseConnection,
    pub ln_address: Arc<dyn LnAddressRepository>,
    pub payment: Arc<dyn PaymentRepository>,
    pub invoice: Arc<dyn InvoiceRepository>,
    pub wallet: Arc<dyn WalletRepository>,
    pub api_key: Arc<dyn ApiKeyRepository>,
    pub config: Arc<dyn ConfigRepository>,
    pub btc_address: Arc<dyn BtcAddressRepository>,
    pub btc_output: Arc<dyn BtcOutputRepository>,
}

impl AppStore {
    pub fn new_sea_orm(db_conn: DatabaseConnection) -> Self {
        let ln_address_repo = SeaOrmLnAddressRepository::new(db_conn.clone());
        let payment_repo = SeaOrmPaymentRepository::new(db_conn.clone());
        let invoice_repo = SeaOrmInvoiceRepository::new(db_conn.clone());
        let wallet_repo = SeaOrmWalletRepository::new(db_conn.clone());
        let api_key_repo = SeaOrmApiKeyRepository::new(db_conn.clone());
        let config_repo = SeaOrmConfigRepository::new(db_conn.clone());
        let btc_address_repo = SeaOrmBitcoinAddressRepository::new(db_conn.clone());
        let btc_output_repo = SeaOrmBitcoinOutputRepository::new(db_conn.clone());

        AppStore {
            db_conn,
            ln_address: Arc::new(ln_address_repo),
            payment: Arc::new(payment_repo),
            invoice: Arc::new(invoice_repo),
            wallet: Arc::new(wallet_repo),
            api_key: Arc::new(api_key_repo),
            config: Arc::new(config_repo),
            btc_address: Arc::new(btc_address_repo),
            btc_output: Arc::new(btc_output_repo),
        }
    }
}

/// Test-only builder for AppStore service tests.
///
/// Configure the public generated mocks, then call `build` to move them into
/// the `Arc<dyn ...>` fields expected by services.
#[cfg(test)]
pub struct AppStoreMockBuilder {
    pub ln_address: MockLnAddressRepository,
    pub payment: MockPaymentRepository,
    pub invoice: MockInvoiceRepository,
    pub wallet: MockWalletRepository,
    pub api_key: MockApiKeyRepository,
    pub config: MockConfigRepository,
    pub btc_address: MockBtcAddressRepository,
    pub btc_output: MockBtcOutputRepository,
}

#[cfg(test)]
impl Default for AppStoreMockBuilder {
    fn default() -> Self {
        Self {
            ln_address: MockLnAddressRepository::new(),
            payment: MockPaymentRepository::new(),
            invoice: MockInvoiceRepository::new(),
            wallet: MockWalletRepository::new(),
            api_key: MockApiKeyRepository::new(),
            config: MockConfigRepository::new(),
            btc_address: MockBtcAddressRepository::new(),
            btc_output: MockBtcOutputRepository::new(),
        }
    }
}

#[cfg(test)]
impl AppStoreMockBuilder {
    pub fn build(self) -> AppStore {
        AppStore {
            db_conn: DatabaseConnection::Disconnected,
            ln_address: Arc::new(self.ln_address),
            payment: Arc::new(self.payment),
            invoice: Arc::new(self.invoice),
            wallet: Arc::new(self.wallet),
            api_key: Arc::new(self.api_key),
            config: Arc::new(self.config),
            btc_address: Arc::new(self.btc_address),
            btc_output: Arc::new(self.btc_output),
        }
    }
}

impl AppStore {
    #[cfg(test)]
    pub fn mock() -> AppStoreMockBuilder {
        AppStoreMockBuilder::default()
    }

    pub async fn begin(&self) -> Result<Option<DatabaseTransaction>, DatabaseError> {
        #[cfg(test)]
        if matches!(self.db_conn, DatabaseConnection::Disconnected) {
            return Ok(None);
        }

        self.db_conn
            .begin()
            .await
            .map(Some)
            .map_err(|e| DatabaseError::Transaction(e.to_string()))
    }

    pub async fn ping(&self) -> Result<(), DatabaseError> {
        self.db_conn
            .ping()
            .await
            .map_err(|e| DatabaseError::Ping(e.to_string()))
    }
}
