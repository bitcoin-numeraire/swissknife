use async_trait::async_trait;

use crate::{application::errors::ApplicationError, domains::invoice::Invoice};

use super::{LnInvoicePaidEvent, LnPayFailureEvent, LnPaySuccessEvent};

#[async_trait]
pub trait LnEventsUseCases: Send + Sync {
    async fn latest_settled_invoice(&self) -> Result<Option<Invoice>, ApplicationError>;
    async fn invoice_paid(&self, event: LnInvoicePaidEvent) -> Result<(), ApplicationError>;
    async fn outgoing_payment(&self, event: LnPaySuccessEvent) -> Result<(), ApplicationError>;
    async fn failed_payment(&self, event: LnPayFailureEvent) -> Result<(), ApplicationError>;
}
