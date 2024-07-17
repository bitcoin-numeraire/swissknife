use std::sync::Arc;

use sea_orm::{DatabaseConnection, DatabaseTransaction, TransactionTrait};

use crate::{
    application::errors::DatabaseError,
    domains::{
        invoice::InvoiceRepository, ln_address::LnAddressRepository, payment::PaymentRepository,
        wallet::WalletRepository,
    },
    infra::database::sea_orm::{
        SeaOrmInvoiceRepository, SeaOrmLnAddressRepository, SeaOrmPaymentRepository,
        SeaOrmWalletRepository,
    },
};

#[derive(Clone)]
pub struct AppStore {
    db_conn: DatabaseConnection,
    pub ln_address: Arc<dyn LnAddressRepository>,
    pub payment: Arc<dyn PaymentRepository>,
    pub invoice: Arc<dyn InvoiceRepository>,
    pub wallet: Arc<dyn WalletRepository>,
}

impl AppStore {
    pub fn new_sea_orm(db_conn: DatabaseConnection) -> Self {
        let ln_address_repo = SeaOrmLnAddressRepository::new(db_conn.clone());
        let payment_repo = SeaOrmPaymentRepository::new(db_conn.clone());
        let invoice_repo = SeaOrmInvoiceRepository::new(db_conn.clone());
        let wallet_repo = SeaOrmWalletRepository::new(db_conn.clone());

        AppStore {
            db_conn,
            ln_address: Arc::new(ln_address_repo),
            payment: Arc::new(payment_repo),
            invoice: Arc::new(invoice_repo),
            wallet: Arc::new(wallet_repo),
        }
    }
}

impl AppStore {
    pub async fn begin(&self) -> Result<DatabaseTransaction, DatabaseError> {
        self.db_conn
            .begin()
            .await
            .map_err(|e| DatabaseError::Transaction(e.to_string()))
    }

    pub async fn ping(&self) -> Result<(), DatabaseError> {
        self.db_conn
            .ping()
            .await
            .map_err(|e| DatabaseError::Ping(e.to_string()))
    }
}
