use async_trait::async_trait;
use sea_orm::{DatabaseConnection, TransactionTrait};

use crate::{
    application::errors::{ApplicationError, DataError, DatabaseError},
    domains::{
        bitcoin::{BtcAddress, BtcAddressRepository, BtcOutput, BtcOutputRepository},
        event::EventProjectionUnitOfWork,
        invoice::{Invoice, InvoiceRepository},
        payment::{Payment, PaymentRepository, PaymentUnitOfWork},
        wallet::WalletBalanceRepository,
    },
};

use super::{
    SeaOrmBitcoinAddressRepository, SeaOrmBitcoinOutputRepository, SeaOrmInvoiceRepository, SeaOrmPaymentRepository,
    SeaOrmWalletBalanceRepository,
};

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
        let balance_repo = SeaOrmWalletBalanceRepository::new(&txn);

        let settled = if invoice.id.is_nil() {
            // New, already-settled incoming invoice (e.g. an on-chain deposit first seen confirmed).
            if let Some(received_msat) = invoice.amount_received_msat {
                balance_repo
                    .credit(invoice.wallet_id, &invoice.currency, received_msat)
                    .await?;
            }
            invoice_repo.insert(invoice).await?
        } else if invoice_repo.settle(&invoice).await? {
            // Pending invoice settled now: credit the receiver exactly once.
            if let Some(received_msat) = invoice.amount_received_msat {
                balance_repo
                    .credit(invoice.wallet_id, &invoice.currency, received_msat)
                    .await?;
            }
            invoice_repo
                .find(invoice.id)
                .await?
                .ok_or_else(|| DataError::NotFound("Invoice not found.".to_string()))?
        } else {
            // Already settled: idempotent replay, no credit.
            invoice_repo
                .find(invoice.id)
                .await?
                .ok_or_else(|| DataError::NotFound("Invoice not found.".to_string()))?
        };

        txn.commit()
            .await
            .map_err(|e| DatabaseError::Transaction(e.to_string()))?;

        Ok(settled)
    }

    async fn project_onchain_deposit(
        &self,
        output: BtcOutput,
        address: BtcAddress,
        mut deposit_invoice: Invoice,
    ) -> Result<Invoice, ApplicationError> {
        let txn = self
            .db
            .begin()
            .await
            .map_err(|e| DatabaseError::Transaction(e.to_string()))?;

        let output_repo = SeaOrmBitcoinOutputRepository::new(&txn);
        let address_repo = SeaOrmBitcoinAddressRepository::new(&txn);
        let invoice_repo = SeaOrmInvoiceRepository::new(&txn);
        let balance_repo = SeaOrmWalletBalanceRepository::new(&txn);

        let stored_output = output_repo.upsert(output).await?;

        if !address.used {
            address_repo.mark_used(address.id).await?;
        }

        // The caller only sets payment_time/amount_received once the deposit is confirmed.
        let confirmed = deposit_invoice.payment_time.is_some();

        let invoice = match invoice_repo.find_by_btc_output_id(stored_output.id).await? {
            Some(mut existing) => {
                if confirmed {
                    // Confirm the previously-pending deposit invoice exactly once.
                    existing.payment_time = deposit_invoice.payment_time;
                    existing.amount_received_msat = deposit_invoice.amount_received_msat;
                    if invoice_repo.settle(&existing).await? {
                        if let Some(received_msat) = existing.amount_received_msat {
                            balance_repo
                                .credit(existing.wallet_id, &existing.currency, received_msat)
                                .await?;
                        }
                    }
                    invoice_repo
                        .find(existing.id)
                        .await?
                        .ok_or_else(|| DataError::NotFound("Invoice not found.".to_string()))?
                } else {
                    // Still unconfirmed: keep the invoice linked to the (re-)seen output.
                    existing.btc_output_id = Some(stored_output.id);
                    invoice_repo.update(existing).await?
                }
            }
            None => {
                deposit_invoice.btc_output_id = Some(stored_output.id);
                if confirmed {
                    if let Some(received_msat) = deposit_invoice.amount_received_msat {
                        balance_repo
                            .credit(deposit_invoice.wallet_id, &deposit_invoice.currency, received_msat)
                            .await?;
                    }
                }
                invoice_repo.insert(deposit_invoice).await?
            }
        };

        txn.commit()
            .await
            .map_err(|e| DatabaseError::Transaction(e.to_string()))?;

        Ok(invoice)
    }
}
