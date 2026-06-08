use std::sync::Arc;

use async_trait::async_trait;
use sea_orm::{DatabaseConnection, TransactionTrait};

use crate::{
    application::errors::{ApplicationError, DataError, DatabaseError},
    domains::{
        payment::{Payment, PaymentUnitOfWork},
        system::HealthProbe,
        wallet::WalletRepository,
    },
};

use super::{
    SeaOrmApiKeyRepository, SeaOrmBitcoinAddressRepository, SeaOrmBitcoinOutputRepository, SeaOrmConfigRepository,
    SeaOrmInvoiceRepository, SeaOrmLnAddressRepository, SeaOrmPaymentRepository, SeaOrmWalletRepository,
};
use crate::application::entities::AppStore;
use crate::domains::payment::PaymentRepository;

pub fn sea_orm_store(db_conn: DatabaseConnection) -> AppStore {
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
        Arc::new(SeaOrmPaymentUnitOfWork::new(db_conn)),
    )
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

#[derive(Clone)]
pub struct SeaOrmPaymentUnitOfWork {
    db: DatabaseConnection,
}

impl SeaOrmPaymentUnitOfWork {
    pub fn new(db: DatabaseConnection) -> Self {
        Self { db }
    }
}

#[async_trait]
impl PaymentUnitOfWork for SeaOrmPaymentUnitOfWork {
    async fn insert_payment(&self, payment: Payment, fee_buffer: f64) -> Result<Payment, ApplicationError> {
        let txn = self
            .db
            .begin()
            .await
            .map_err(|e| DatabaseError::Transaction(e.to_string()))?;

        let wallet_repo = SeaOrmWalletRepository::new(self.db.clone());
        let payment_repo = SeaOrmPaymentRepository::new(self.db.clone());

        let balance = wallet_repo
            .get_balance(Some(&txn), payment.wallet_id)
            .await?
            .available_msat as f64;

        let required_balance_msat = if let Some(fee_msat) = payment.fee_msat {
            (payment.amount_msat.saturating_add(fee_msat)) as f64
        } else {
            payment.amount_msat as f64 * (1.0 + fee_buffer)
        };

        if balance < required_balance_msat {
            return Err(DataError::InsufficientFunds(required_balance_msat).into());
        }

        let pending_payment = payment_repo.insert(Some(&txn), payment).await?;

        txn.commit()
            .await
            .map_err(|e| DatabaseError::Transaction(e.to_string()))?;

        Ok(pending_payment)
    }
}
