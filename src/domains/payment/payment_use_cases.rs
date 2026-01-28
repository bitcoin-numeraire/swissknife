use async_trait::async_trait;
use uuid::Uuid;

use crate::application::errors::ApplicationError;

use super::{Payment, PaymentFilter};

#[async_trait]
pub trait PaymentsUseCases: Send + Sync {
    async fn pay(
        &self,
        input: String,
        amount_msat: Option<u64>,
        comment: Option<String>,
        wallet_id: Uuid,
    ) -> Result<Payment, ApplicationError>;
    async fn get(&self, id: Uuid) -> Result<Payment, ApplicationError>;
    async fn list(&self, filter: PaymentFilter) -> Result<Vec<Payment>, ApplicationError>;
    async fn delete(&self, id: Uuid) -> Result<(), ApplicationError>;
    async fn delete_many(&self, filter: PaymentFilter) -> Result<u64, ApplicationError>;
}
