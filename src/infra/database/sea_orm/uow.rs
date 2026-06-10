use async_trait::async_trait;
use sea_orm::{DatabaseConnection, TransactionTrait};

use crate::{
    application::errors::{ApplicationError, DataError, DatabaseError},
    domains::{
        event::EventProjectionUnitOfWork,
        invoice::{Invoice, InvoiceRepository},
        payment::{Payment, PaymentRepository, PaymentUnitOfWork},
        wallet::WalletBalanceRepository,
    },
};

use super::{SeaOrmInvoiceRepository, SeaOrmPaymentRepository, SeaOrmWalletBalanceRepository};

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
            return Err(
                DataError::Inconsistency(format!("Reserved balance missing for payment {}", payment.id)).into(),
            );
        }

        let actual_msat = payment.amount_msat.saturating_add(payment.fee_msat.unwrap_or_default());
        if actual_msat > 0
            && !balance_repo
                .debit(payment.wallet_id, &payment.currency, actual_msat)
                .await?
        {
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
            return Err(
                DataError::Inconsistency(format!("Reserved balance missing for payment {}", payment.id)).into(),
            );
        }
        payment.reserved_amount = 0;

        let payment = SeaOrmPaymentRepository::new(&txn).update(payment).await?;

        txn.commit()
            .await
            .map_err(|e| DatabaseError::Transaction(e.to_string()))?;

        Ok(payment)
    }

    async fn settle_internal(&self, mut payment: Payment, invoice: Invoice) -> Result<Payment, ApplicationError> {
        let txn = self
            .db
            .begin()
            .await
            .map_err(|e| DatabaseError::Transaction(e.to_string()))?;

        let balance_repo = SeaOrmWalletBalanceRepository::new(&txn);

        // Debit the sender first so an underfunded sender fails before the receiver is credited.
        let debit_msat = payment.amount_msat.saturating_add(payment.fee_msat.unwrap_or_default());
        if debit_msat > 0
            && !balance_repo
                .debit(payment.wallet_id, &payment.currency, debit_msat)
                .await?
        {
            return Err(DataError::InsufficientFunds(debit_msat as f64).into());
        }
        payment.reserved_amount = 0;
        let payment = SeaOrmPaymentRepository::new(&txn).insert(payment).await?;

        if let Some(received_msat) = invoice.amount_received_msat {
            balance_repo
                .credit(invoice.wallet_id, &invoice.currency, received_msat)
                .await?;
        }

        let invoice_repo = SeaOrmInvoiceRepository::new(&txn);
        if invoice.id.is_nil() {
            invoice_repo.insert(invoice).await?;
        } else {
            invoice_repo.update(invoice).await?;
        }

        txn.commit()
            .await
            .map_err(|e| DatabaseError::Transaction(e.to_string()))?;

        Ok(payment)
    }
}

#[derive(Clone)]
pub struct SeaOrmEventProjectionUnitOfWork {
    db: DatabaseConnection,
}

impl SeaOrmEventProjectionUnitOfWork {
    pub fn new(db: DatabaseConnection) -> Self {
        Self { db }
    }
}

#[async_trait]
impl EventProjectionUnitOfWork for SeaOrmEventProjectionUnitOfWork {
    async fn settle_incoming_invoice(&self, invoice: Invoice) -> Result<Invoice, ApplicationError> {
        let txn = self
            .db
            .begin()
            .await
            .map_err(|e| DatabaseError::Transaction(e.to_string()))?;

        let invoice_repo = SeaOrmInvoiceRepository::new(&txn);

        // Conditional settle applies once; a replayed event finds it already settled and skips the credit.
        if !invoice_repo.settle(&invoice).await? {
            let existing = invoice_repo
                .find(invoice.id)
                .await?
                .ok_or_else(|| DataError::NotFound("Invoice not found.".to_string()))?;
            txn.commit()
                .await
                .map_err(|e| DatabaseError::Transaction(e.to_string()))?;
            return Ok(existing);
        }

        if let Some(received_msat) = invoice.amount_received_msat {
            SeaOrmWalletBalanceRepository::new(&txn)
                .credit(invoice.wallet_id, &invoice.currency, received_msat)
                .await?;
        }

        let settled = invoice_repo
            .find(invoice.id)
            .await?
            .ok_or_else(|| DataError::NotFound("Invoice not found.".to_string()))?;

        txn.commit()
            .await
            .map_err(|e| DatabaseError::Transaction(e.to_string()))?;

        Ok(settled)
    }
}
