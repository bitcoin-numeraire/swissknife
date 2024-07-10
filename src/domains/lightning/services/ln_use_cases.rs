use async_trait::async_trait;

use uuid::Uuid;

use crate::{
    application::errors::ApplicationError,
    domains::{
        invoices::entities::Invoice,
        lightning::entities::{
            LnAddress, LnAddressFilter, LnInvoicePaidEvent, LnPayFailureEvent, LnPaySuccessEvent,
            LnURLPayRequest, LnUrlCallback,
        },
    },
};

#[async_trait]
pub trait LnUrlUseCases: Send + Sync {
    async fn lnurlp(&self, username: String) -> Result<LnURLPayRequest, ApplicationError>;
    async fn lnurlp_callback(
        &self,
        username: String,
        amount: u64,
        comment: Option<String>,
    ) -> Result<LnUrlCallback, ApplicationError>;
    async fn register(
        &self,
        user_id: String,
        username: String,
    ) -> Result<LnAddress, ApplicationError>;
    async fn get(&self, id: Uuid) -> Result<LnAddress, ApplicationError>;
    async fn list(&self, filter: LnAddressFilter) -> Result<Vec<LnAddress>, ApplicationError>;
    async fn delete(&self, id: Uuid) -> Result<(), ApplicationError>;
    async fn delete_many(&self, filter: LnAddressFilter) -> Result<u64, ApplicationError>;
}

#[async_trait]
pub trait LnEventsUseCases: Send + Sync {
    async fn latest_settled_invoice(&self) -> Result<Option<Invoice>, ApplicationError>;
    async fn invoice_paid(&self, event: LnInvoicePaidEvent) -> Result<(), ApplicationError>;
    async fn outgoing_payment(&self, event: LnPaySuccessEvent) -> Result<(), ApplicationError>;
    async fn failed_payment(&self, event: LnPayFailureEvent) -> Result<(), ApplicationError>;
}
