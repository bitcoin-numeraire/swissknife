use async_trait::async_trait;
use breez_sdk_core::{Payment, PaymentFailedData};

use crate::{
    application::errors::ApplicationError,
    domains::lightning::entities::{Invoice, LightningPayment},
};

#[async_trait]
pub trait PaymentsProcessorUseCases: Send + Sync {
    async fn process_incoming_payment(
        &self,
        payment: Payment,
    ) -> Result<Invoice, ApplicationError>;
    async fn process_outgoing_payment(
        &self,
        payment: Payment,
    ) -> Result<LightningPayment, ApplicationError>;
    async fn process_failed_payment(
        &self,
        payment: PaymentFailedData,
    ) -> Result<LightningPayment, ApplicationError>;
}
