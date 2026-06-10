use async_trait::async_trait;

use crate::application::errors::ApplicationError;

use super::Payment;

#[cfg_attr(test, mockall::automock)]
#[async_trait]
pub trait PaymentUnitOfWork: Send + Sync {
    // TODO(cutover): `insert_payment` is the pre-reservation path; it is removed once every send
    // flow is migrated to `reserve`/`settle`/`fail` (and the upcoming `settle_internal`).
    async fn insert_payment(&self, payment: Payment, fee_buffer: f64) -> Result<Payment, ApplicationError>;

    /// Reserve `reserve_amount_msat` and insert a pending outgoing payment, atomically.
    async fn reserve(&self, payment: Payment, reserve_amount_msat: u64) -> Result<Payment, ApplicationError>;

    /// Settle a reserved payment: release the reservation and debit the actual spend, atomically.
    async fn settle(&self, payment: Payment) -> Result<Payment, ApplicationError>;

    /// Fail a reserved payment: release the reservation, atomically.
    async fn fail(&self, payment: Payment) -> Result<Payment, ApplicationError>;
}
