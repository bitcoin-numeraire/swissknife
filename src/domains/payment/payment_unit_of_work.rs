use async_trait::async_trait;

use crate::application::errors::ApplicationError;

use super::Payment;

#[cfg_attr(test, mockall::automock)]
#[async_trait]
pub trait PaymentUnitOfWork: Send + Sync {
    async fn insert_payment(&self, payment: Payment, fee_buffer: f64) -> Result<Payment, ApplicationError>;
}
