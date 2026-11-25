use async_trait::async_trait;

use crate::{application::errors::ApplicationError, domains::invoice::Invoice};

use super::{ClaimedPayment, Payment, PaymentInstructions, SubmissionKey, SubmittedPayment};

#[cfg_attr(test, mockall::automock)]
#[async_trait]
pub trait PaymentUnitOfWork: Send + Sync {
    async fn submission_by_id(
        &self,
        wallet_id: Option<uuid::Uuid>,
        id: uuid::Uuid,
    ) -> Result<Option<SubmittedPayment>, ApplicationError>;

    /// Return the original result for a matching wallet-scoped request.
    async fn submitted(&self, key: &SubmissionKey) -> Result<Option<SubmittedPayment>, ApplicationError>;

    /// Reserve funds, attach provider instructions and publish acceptance in one database transaction.
    async fn submit(
        &self,
        key: &SubmissionKey,
        payment: Payment,
        reserve_amount: u64,
        instructions: PaymentInstructions,
    ) -> Result<SubmittedPayment, ApplicationError>;

    /// Settle a registered recipient and its wallet-scoped idempotency key in the same transaction.
    async fn submit_internal(
        &self,
        key: &SubmissionKey,
        payment: Payment,
        invoice: Invoice,
    ) -> Result<SubmittedPayment, ApplicationError>;

    /// Commit the dispatch boundary before contacting the provider. Only one worker can claim a command.
    async fn claim(&self) -> Result<Option<ClaimedPayment>, ApplicationError>;

    async fn claim_withdrawal(&self, backend: &str) -> Result<Option<super::ClaimedWithdrawal>, ApplicationError>;
    async fn advance_withdrawal(
        &self,
        claim: &super::ClaimedWithdrawal,
        progress: super::WithdrawalProgress,
        retry_seconds: i64,
    ) -> Result<bool, ApplicationError>;
    async fn cancel_withdrawal(&self, wallet_id: Option<uuid::Uuid>, id: uuid::Uuid) -> Result<(), ApplicationError>;

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
