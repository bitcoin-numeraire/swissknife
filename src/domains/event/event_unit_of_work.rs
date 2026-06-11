use async_trait::async_trait;

use crate::{
    application::errors::ApplicationError,
    domains::{
        bitcoin::{BtcAddress, BtcOutput},
        invoice::Invoice,
    },
};

#[cfg_attr(test, mockall::automock)]
#[async_trait]
pub trait EventProjectionUnitOfWork: Send + Sync {
    /// Settle an incoming invoice and credit the receiver's wallet balance in one transaction.
    /// Idempotent: a replayed settle event credits the wallet at most once.
    async fn settle_incoming_invoice(&self, invoice: Invoice) -> Result<Invoice, ApplicationError>;

    /// Project an on-chain deposit in one transaction: upsert the output, mark the receiving
    /// address used, and settle-or-insert the linked invoice (crediting the receiver when
    /// confirmed). `deposit_invoice` is the desired invoice for a first sighting; when an invoice
    /// already exists for the output it is settled idempotently instead.
    async fn project_onchain_deposit(
        &self,
        output: BtcOutput,
        address: BtcAddress,
        deposit_invoice: Invoice,
    ) -> Result<Invoice, ApplicationError>;
}
