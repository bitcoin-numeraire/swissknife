use async_trait::async_trait;

use crate::application::entities::Currency;
use crate::application::errors::ApplicationError;
use crate::domains::invoice::Invoice;

use super::{BtcOutputEvent, BtcWithdrawalConfirmedEvent, LnInvoicePaidEvent, LnPayFailureEvent, LnPaySuccessEvent};

#[async_trait]
pub trait EventUseCases: Send + Sync {
    async fn latest_settled_invoice(&self) -> Result<Option<Invoice>, ApplicationError>;
    async fn invoice_paid(&self, event: LnInvoicePaidEvent) -> Result<(), ApplicationError>;
    async fn outgoing_payment(&self, event: LnPaySuccessEvent) -> Result<(), ApplicationError>;
    async fn failed_payment(&self, event: LnPayFailureEvent) -> Result<(), ApplicationError>;
    async fn onchain_deposit(&self, event: BtcOutputEvent, currency: Currency) -> Result<(), ApplicationError>;
    async fn onchain_withdrawal(&self, event: BtcOutputEvent) -> Result<(), ApplicationError>;
    async fn onchain_withdrawal_confirmed(&self, event: BtcWithdrawalConfirmedEvent) -> Result<(), ApplicationError>;
}
