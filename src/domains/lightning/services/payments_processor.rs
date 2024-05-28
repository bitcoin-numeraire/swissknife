use async_trait::async_trait;
use breez_sdk_core::{Payment as BreezPayment, PaymentFailedData};

use crate::{
    application::errors::ApplicationError,
    domains::lightning::entities::{Invoice, Payment},
};

#[async_trait]
pub trait PaymentsProcessorUseCases: Send + Sync {
    async fn process_incoming_payment(
        &self,
        payment: BreezPayment,
    ) -> Result<Invoice, ApplicationError>;
    async fn process_outgoing_payment(
        &self,
        payment: BreezPayment,
    ) -> Result<Payment, ApplicationError>;
    async fn process_failed_payment(
        &self,
        payment: PaymentFailedData,
    ) -> Result<Payment, ApplicationError>;
}
