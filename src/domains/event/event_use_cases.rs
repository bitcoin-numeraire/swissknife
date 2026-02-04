use async_trait::async_trait;

use crate::application::entities::Currency;
use crate::application::errors::ApplicationError;

use super::{BtcOutputEvent, LnInvoicePaidEvent, LnPayFailureEvent, LnPaySuccessEvent};

#[async_trait]
pub trait EventUseCases: Send + Sync {
    async fn invoice_paid(&self, event: LnInvoicePaidEvent) -> Result<(), ApplicationError>;
    async fn outgoing_payment(&self, event: LnPaySuccessEvent) -> Result<(), ApplicationError>;
    async fn failed_payment(&self, event: LnPayFailureEvent) -> Result<(), ApplicationError>;
    async fn onchain_deposit(&self, event: BtcOutputEvent, currency: Currency) -> Result<(), ApplicationError>;
    async fn onchain_withdrawal(&self, event: BtcOutputEvent) -> Result<(), ApplicationError>;
}
