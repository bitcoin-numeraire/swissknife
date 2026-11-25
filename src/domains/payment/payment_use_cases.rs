use async_trait::async_trait;
use uuid::Uuid;

use crate::application::errors::ApplicationError;

use super::{Payment, PaymentFeeEstimate, PaymentFilter};

#[cfg_attr(test, mockall::automock)]
#[async_trait]
pub trait PaymentsUseCases: Send + Sync {
    async fn submit_payment(
        &self,
        wallet_id: Uuid,
        agent_key_id: Option<Uuid>,
        request: swissknife_types::SubmitPaymentRequest,
    ) -> Result<super::SubmittedPayment, ApplicationError>;
    async fn submission(&self, wallet_id: Option<Uuid>, id: Uuid) -> Result<super::SubmittedPayment, ApplicationError>;
    async fn cancel_withdrawal(
        &self,
        wallet_id: Option<Uuid>,
        id: Uuid,
    ) -> Result<super::SubmittedPayment, ApplicationError>;
    async fn execute_next_withdrawal(&self) -> Result<bool, ApplicationError>;
    async fn execute_next(&self) -> Result<bool, ApplicationError>;

    async fn estimate_fee(
        &self,
        input: String,
        amount_msat: Option<u64>,
        comment: Option<String>,
        wallet_id: Uuid,
    ) -> Result<PaymentFeeEstimate, ApplicationError>;
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
    async fn sync(&self) -> Result<u32, ApplicationError>;
    /// Synchronize one known payment by its payment hash.
    async fn sync_one(&self, payment_hash: String) -> Result<(), ApplicationError>;
}
