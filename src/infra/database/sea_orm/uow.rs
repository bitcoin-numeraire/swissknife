use async_trait::async_trait;
use sea_orm::{DatabaseConnection, TransactionTrait};

use crate::{
    application::errors::{ApplicationError, DataError, DatabaseError},
    domains::{
        bitcoin::{BtcAddress, BtcAddressRepository, BtcOutput, BtcOutputRepository},
        event::{ClientEventRepository, EventProjectionUnitOfWork, NewClientEvent},
        invoice::{Invoice, InvoiceRepository},
        payment::{Payment, PaymentRepository, PaymentStatus, PaymentUnitOfWork},
        wallet::WalletRepository,
    },
};

use super::{
    SeaOrmBitcoinAddressRepository, SeaOrmBitcoinOutputRepository, SeaOrmClientEventRepository,
    SeaOrmInvoiceRepository, SeaOrmPaymentRepository, SeaOrmWalletRepository,
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

        let wallet_repo = SeaOrmWalletRepository::new(&txn);
        if !wallet_repo.reserve(payment.wallet_id, reserve_amount_msat).await? {
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

        let payment_repo = SeaOrmPaymentRepository::new(&txn);

        // Single-winner: the synchronous pay result and the success event can
        // both reach this for the same payment, so only the caller that wins the
        // transition to Settled runs the balance side effects. `Failed` is an
        // accepted source: a premature error (e.g. an RPC timeout) can mark a
        // payment failed and release its reservation while it is still in flight,
        // and a later success must still settle and debit it.
        if !payment_repo
            .try_transition(
                payment.id,
                &[PaymentStatus::Pending, PaymentStatus::Failed],
                PaymentStatus::Settled,
            )
            .await?
        {
            let settled = payment_repo
                .find(payment.id)
                .await?
                .ok_or_else(|| DataError::NotFound(format!("Payment {} not found", payment.id)))?;
            txn.commit()
                .await
                .map_err(|e| DatabaseError::Transaction(e.to_string()))?;
            return Ok(settled);
        }

        // Decide the release from the stored row, not the caller's payload: a
        // premature `fail` racing this success can already have released the
        // reservation (storing reserved_amount = 0) while the in-memory payload
        // still carries the old nonzero amount. Releasing that stale amount would
        // fail and roll back an actual settlement, leaving it marked failed.
        let reserved_amount = payment_repo
            .find(payment.id)
            .await?
            .ok_or_else(|| DataError::NotFound(format!("Payment {} not found", payment.id)))?
            .reserved_amount;

        let wallet_repo = SeaOrmWalletRepository::new(&txn);

        if reserved_amount > 0 && !wallet_repo.release(payment.wallet_id, reserved_amount).await? {
            return Err(
                DataError::Inconsistency(format!("Reserved balance missing for payment {}", payment.id)).into(),
            );
        }

        let actual_msat = payment.amount_msat.saturating_add(payment.fee_msat.unwrap_or_default());
        if actual_msat > 0 && !wallet_repo.debit_confirmed(payment.wallet_id, actual_msat).await? {
            return Err(
                DataError::Inconsistency(format!("Wallet balance missing for settled payment {}", payment.id)).into(),
            );
        }
        payment.reserved_amount = 0;
        // A successful settlement carries no failure reason, even when correcting
        // a payment that was prematurely marked failed.
        payment.error = None;

        let payment = payment_repo.update(payment).await?;

        SeaOrmClientEventRepository::new(&txn)
            .append(NewClientEvent::payment(&payment)?)
            .await?;

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

        let payment_repo = SeaOrmPaymentRepository::new(&txn);

        // Single-winner, and only from Pending: a duplicate failure (sync result
        // + failure event) returns the already-failed payment, and a payment that
        // already settled is never moved back to Failed.
        if !payment_repo
            .try_transition(payment.id, &[PaymentStatus::Pending], PaymentStatus::Failed)
            .await?
        {
            let failed = payment_repo
                .find(payment.id)
                .await?
                .ok_or_else(|| DataError::NotFound(format!("Payment {} not found", payment.id)))?;
            txn.commit()
                .await
                .map_err(|e| DatabaseError::Transaction(e.to_string()))?;
            return Ok(failed);
        }

        let wallet_repo = SeaOrmWalletRepository::new(&txn);
        if payment.reserved_amount > 0 && !wallet_repo.release(payment.wallet_id, payment.reserved_amount).await? {
            return Err(
                DataError::Inconsistency(format!("Reserved balance missing for payment {}", payment.id)).into(),
            );
        }
        payment.reserved_amount = 0;

        let payment = payment_repo.update(payment).await?;

        SeaOrmClientEventRepository::new(&txn)
            .append(NewClientEvent::payment(&payment)?)
            .await?;

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

        let wallet_repo = SeaOrmWalletRepository::new(&txn);
        let invoice_repo = SeaOrmInvoiceRepository::new(&txn);

        let existing_invoice = !invoice.id.is_nil();
        if existing_invoice && !invoice_repo.settle(&invoice).await? {
            return Err(DataError::Conflict("Invoice has already been paid.".to_string()).into());
        }

        let debit_msat = payment.amount_msat.saturating_add(payment.fee_msat.unwrap_or_default());
        if debit_msat > 0 && !wallet_repo.debit(payment.wallet_id, debit_msat).await? {
            return Err(DataError::InsufficientFunds(debit_msat as f64).into());
        }
        payment.reserved_amount = 0;
        let payment = SeaOrmPaymentRepository::new(&txn).insert(payment).await?;

        let invoice = if invoice.id.is_nil() {
            if let Some(received_msat) = invoice.amount_received_msat {
                wallet_repo.credit(invoice.wallet_id, received_msat).await?;
            }
            invoice_repo.insert(invoice).await?
        } else {
            if let Some(received_msat) = invoice.amount_received_msat {
                wallet_repo.credit(invoice.wallet_id, received_msat).await?;
            }
            invoice
        };

        let event_repo = SeaOrmClientEventRepository::new(&txn);
        event_repo.append(NewClientEvent::payment(&payment)?).await?;
        event_repo.append(NewClientEvent::invoice_paid(&invoice)?).await?;

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
        let wallet_repo = SeaOrmWalletRepository::new(&txn);

        let (settled, should_emit) = if invoice.id.is_nil() {
            // New, already-settled incoming invoice (e.g. an on-chain deposit first seen confirmed).
            if let Some(received_msat) = invoice.amount_received_msat {
                wallet_repo.credit(invoice.wallet_id, received_msat).await?;
            }
            (invoice_repo.insert(invoice).await?, true)
        } else if invoice_repo.settle(&invoice).await? {
            // Pending invoice settled now: credit the receiver exactly once.
            if let Some(received_msat) = invoice.amount_received_msat {
                wallet_repo.credit(invoice.wallet_id, received_msat).await?;
            }
            (
                invoice_repo
                    .find(invoice.id)
                    .await?
                    .ok_or_else(|| DataError::NotFound("Invoice not found.".to_string()))?,
                true,
            )
        } else {
            // Already settled: idempotent replay, no credit.
            (
                invoice_repo
                    .find(invoice.id)
                    .await?
                    .ok_or_else(|| DataError::NotFound("Invoice not found.".to_string()))?,
                false,
            )
        };

        if should_emit {
            SeaOrmClientEventRepository::new(&txn)
                .append(NewClientEvent::invoice_paid(&settled)?)
                .await?;
        }

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
        let wallet_repo = SeaOrmWalletRepository::new(&txn);

        let stored_output = output_repo.upsert(output).await?;

        if !address.used {
            address_repo.mark_used(address.id).await?;
        }

        // The caller only sets payment_time/amount_received once the deposit is confirmed.
        let confirmed = deposit_invoice.payment_time.is_some();

        let (invoice, should_emit) = match invoice_repo.find_by_btc_output_id(stored_output.id).await? {
            Some(mut existing) => {
                if confirmed {
                    // Confirm the previously-pending deposit invoice exactly once.
                    existing.payment_time = deposit_invoice.payment_time;
                    existing.amount_received_msat = deposit_invoice.amount_received_msat;
                    let newly_settled = invoice_repo.settle(&existing).await?;
                    if newly_settled {
                        if let Some(received_msat) = existing.amount_received_msat {
                            wallet_repo.credit(existing.wallet_id, received_msat).await?;
                        }
                    }
                    (
                        invoice_repo
                            .find(existing.id)
                            .await?
                            .ok_or_else(|| DataError::NotFound("Invoice not found.".to_string()))?,
                        newly_settled,
                    )
                } else {
                    // Still unconfirmed: keep the invoice linked to the (re-)seen output.
                    existing.btc_output_id = Some(stored_output.id);
                    (invoice_repo.update(existing).await?, false)
                }
            }
            None => {
                deposit_invoice.btc_output_id = Some(stored_output.id);
                if confirmed {
                    if let Some(received_msat) = deposit_invoice.amount_received_msat {
                        wallet_repo.credit(deposit_invoice.wallet_id, received_msat).await?;
                    }
                }
                let invoice = invoice_repo.insert(deposit_invoice).await?;
                (invoice, confirmed)
            }
        };

        if should_emit {
            SeaOrmClientEventRepository::new(&txn)
                .append(NewClientEvent::invoice_paid(&invoice)?)
                .await?;
        }

        txn.commit()
            .await
            .map_err(|e| DatabaseError::Transaction(e.to_string()))?;

        Ok(invoice)
    }
}
