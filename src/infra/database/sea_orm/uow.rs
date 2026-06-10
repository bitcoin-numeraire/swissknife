use async_trait::async_trait;
use sea_orm::{DatabaseConnection, TransactionTrait};

use crate::{
    application::errors::{ApplicationError, DataError, DatabaseError},
    domains::{
        payment::{Payment, PaymentRepository, PaymentUnitOfWork},
        wallet::{WalletBalanceRepository, WalletRepository},
    },
};

use super::{SeaOrmPaymentRepository, SeaOrmWalletBalanceRepository, SeaOrmWalletRepository};

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
    // TODO(cutover): removed once all send flows use `reserve`/`settle`/`fail`.
    async fn insert_payment(&self, payment: Payment, fee_buffer: f64) -> Result<Payment, ApplicationError> {
        let txn = self
            .db
            .begin()
            .await
            .map_err(|e| DatabaseError::Transaction(e.to_string()))?;

        let wallet_repo = SeaOrmWalletRepository::new(&txn);
        let payment_repo = SeaOrmPaymentRepository::new(&txn);

        let balance = wallet_repo.get_balance(payment.wallet_id).await?.available_msat as f64;

        let required_balance_msat = if let Some(fee_msat) = payment.fee_msat {
            (payment.amount_msat.saturating_add(fee_msat)) as f64
        } else {
            payment.amount_msat as f64 * (1.0 + fee_buffer)
        };

        if balance < required_balance_msat {
            return Err(DataError::InsufficientFunds(required_balance_msat).into());
        }

        let pending_payment = payment_repo.insert(payment).await?;

        txn.commit()
            .await
            .map_err(|e| DatabaseError::Transaction(e.to_string()))?;

        Ok(pending_payment)
    }

    async fn reserve(&self, mut payment: Payment, reserve_amount_msat: u64) -> Result<Payment, ApplicationError> {
        let txn = self
            .db
            .begin()
            .await
            .map_err(|e| DatabaseError::Transaction(e.to_string()))?;

        let balance_repo = SeaOrmWalletBalanceRepository::new(&txn);
        if !balance_repo
            .reserve(payment.wallet_id, &payment.currency, reserve_amount_msat)
            .await?
        {
            return Err(DataError::InsufficientFunds(reserve_amount_msat as f64).into());
        }
        payment.reserved_amount = reserve_amount_msat;

        let payment = SeaOrmPaymentRepository::new(&txn).insert(payment).await?;

        txn.commit()
            .await
            .map_err(|e| DatabaseError::Transaction(e.to_string()))?;

        Ok(payment)
    }

    async fn settle(&self, mut payment: Payment) -> Result<Payment, ApplicationError> {
        let txn = self
            .db
            .begin()
            .await
            .map_err(|e| DatabaseError::Transaction(e.to_string()))?;

        let balance_repo = SeaOrmWalletBalanceRepository::new(&txn);

        if payment.reserved_amount > 0
            && !balance_repo
                .release(payment.wallet_id, &payment.currency, payment.reserved_amount)
                .await?
        {
            return Err(DataError::Inconsistency(format!("Reserved balance missing for payment {}", payment.id)).into());
        }

        let actual_msat = payment.amount_msat.saturating_add(payment.fee_msat.unwrap_or_default());
        if actual_msat > 0 && !balance_repo.debit(payment.wallet_id, &payment.currency, actual_msat).await? {
            return Err(DataError::InsufficientFunds(actual_msat as f64).into());
        }
        payment.reserved_amount = 0;

        let payment = SeaOrmPaymentRepository::new(&txn).update(payment).await?;

        txn.commit()
            .await
            .map_err(|e| DatabaseError::Transaction(e.to_string()))?;

        Ok(payment)
    }

    async fn fail(&self, mut payment: Payment) -> Result<Payment, ApplicationError> {
        let txn = self
            .db
            .begin()
            .await
            .map_err(|e| DatabaseError::Transaction(e.to_string()))?;

        let balance_repo = SeaOrmWalletBalanceRepository::new(&txn);
        if payment.reserved_amount > 0
            && !balance_repo
                .release(payment.wallet_id, &payment.currency, payment.reserved_amount)
                .await?
        {
            return Err(DataError::Inconsistency(format!("Reserved balance missing for payment {}", payment.id)).into());
        }
        payment.reserved_amount = 0;

        let payment = SeaOrmPaymentRepository::new(&txn).update(payment).await?;

        txn.commit()
            .await
            .map_err(|e| DatabaseError::Transaction(e.to_string()))?;

        Ok(payment)
    }
}
