use async_trait::async_trait;
use breez_sdk_core::{Payment as BreezPayment, PaymentFailedData};
use uuid::Uuid;

use crate::{
    application::{dtos::SendPaymentRequest, errors::ApplicationError},
    domains::payments::entities::{Payment, PaymentFilter},
};
#[async_trait]
pub trait PaymentsUseCases: Send + Sync {
    async fn pay(&self, req: SendPaymentRequest) -> Result<Payment, ApplicationError>;
    async fn get(&self, id: Uuid) -> Result<Payment, ApplicationError>;
    async fn list(&self, filter: PaymentFilter) -> Result<Vec<Payment>, ApplicationError>;
    async fn delete(&self, id: Uuid) -> Result<(), ApplicationError>;
    async fn delete_many(&self, filter: PaymentFilter) -> Result<u64, ApplicationError>;
}

#[async_trait]
pub trait LnEventsUseCases: Send + Sync {
    async fn process_incoming_payment(&self, payment: BreezPayment)
        -> Result<(), ApplicationError>;
    async fn process_outgoing_payment(&self, payment: BreezPayment)
        -> Result<(), ApplicationError>;
    async fn process_failed_payment(
        &self,
        payment: PaymentFailedData,
    ) -> Result<(), ApplicationError>;
}
