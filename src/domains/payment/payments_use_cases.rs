use async_trait::async_trait;
use uuid::Uuid;

use crate::application::{dtos::SendPaymentRequest, errors::ApplicationError};

use super::{Payment, PaymentFilter};

#[async_trait]
pub trait PaymentsUseCases: Send + Sync {
    async fn pay(&self, req: SendPaymentRequest) -> Result<Payment, ApplicationError>;
    async fn get(&self, id: Uuid) -> Result<Payment, ApplicationError>;
    async fn list(&self, filter: PaymentFilter) -> Result<Vec<Payment>, ApplicationError>;
    async fn delete(&self, id: Uuid) -> Result<(), ApplicationError>;
    async fn delete_many(&self, filter: PaymentFilter) -> Result<u64, ApplicationError>;
}
