use std::sync::Arc;

use sea_orm::DatabaseConnection;

use crate::domains::{
    invoices::adapters::{InvoiceRepository, SeaOrmInvoiceRepository},
    lightning::adapters::{LightningRepository, SqlxStore},
    payments::adapters::{PaymentRepository, SeaOrmPaymentRepository},
    users::adapters::{SeaOrmUserRepository, UserRepository},
};

#[derive(Clone)]
pub struct AppStore {
    pub lightning: Arc<dyn LightningRepository>,
    pub payment: Arc<dyn PaymentRepository>,
    pub invoice: Arc<dyn InvoiceRepository>,
    pub user: Arc<dyn UserRepository>,
}

impl AppStore {
    pub fn new_sea_orm(db_conn: DatabaseConnection) -> Self {
        let lightning_repo = SqlxStore::new(db_conn.clone());
        let payment_repo = SeaOrmPaymentRepository::new(db_conn.clone());
        let invoice_repo = SeaOrmInvoiceRepository::new(db_conn.clone());
        let user_repo = SeaOrmUserRepository::new(db_conn);

        AppStore {
            lightning: Arc::new(lightning_repo),
            payment: Arc::new(payment_repo),
            invoice: Arc::new(invoice_repo),
            user: Arc::new(user_repo),
        }
    }
}
