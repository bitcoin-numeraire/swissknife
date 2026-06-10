use async_trait::async_trait;

use crate::{application::errors::ApplicationError, domains::invoice::Invoice};

#[cfg_attr(test, mockall::automock)]
#[async_trait]
pub trait EventProjectionUnitOfWork: Send + Sync {
    /// Settle an incoming invoice and credit the receiver's wallet balance in one transaction.
    /// Idempotent: a replayed settle event credits the wallet at most once.
    async fn settle_incoming_invoice(&self, invoice: Invoice) -> Result<Invoice, ApplicationError>;
}
