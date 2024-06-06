use async_trait::async_trait;

use uuid::Uuid;

use crate::{
    application::errors::ApplicationError,
    domains::invoices::entities::{Invoice, InvoiceFilter, LnURLpInvoice},
};

#[async_trait]
pub trait InvoiceUseCases: Send + Sync {
    async fn invoice(
        &self,
        user_id: String,
        amount: u64,
        description: Option<String>,
        expiry: Option<u32>,
    ) -> Result<Invoice, ApplicationError>;
    async fn invoice_lnurlp(
        &self,
        username: String,
        amount: u64,
        comment: Option<String>,
    ) -> Result<LnURLpInvoice, ApplicationError>;
    async fn get(&self, id: Uuid) -> Result<Invoice, ApplicationError>;
    async fn list(&self, filter: InvoiceFilter) -> Result<Vec<Invoice>, ApplicationError>;
    async fn delete(&self, id: Uuid) -> Result<(), ApplicationError>;
    async fn delete_many(&self, filter: InvoiceFilter) -> Result<u64, ApplicationError>;
}