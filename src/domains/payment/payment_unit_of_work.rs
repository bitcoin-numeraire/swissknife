use async_trait::async_trait;

use crate::{application::errors::ApplicationError, domains::invoice::Invoice};

use super::Payment;

#[cfg_attr(test, mockall::automock)]
#[async_trait]
pub trait PaymentUnitOfWork: Send + Sync {
    /// Reserve `reserve_amount_msat` and insert a pending outgoing payment, atomically.
    async fn reserve(&self, payment: Payment, reserve_amount_msat: u64) -> Result<Payment, ApplicationError>;

    /// Settle a reserved payment: release the reservation and debit the actual spend, atomically.
    async fn settle(&self, payment: Payment) -> Result<Payment, ApplicationError>;

    /// Fail a reserved payment: release the reservation, atomically.
    async fn fail(&self, payment: Payment) -> Result<Payment, ApplicationError>;

    /// Settle an internal payment: debit the sender, credit the receiver, and insert the payment
    /// and its counterpart invoice, atomically.
    async fn settle_internal(&self, payment: Payment, invoice: Invoice) -> Result<Payment, ApplicationError>;
}
