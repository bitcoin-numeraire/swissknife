use async_trait::async_trait;

use uuid::Uuid;

use crate::{
    application::errors::ApplicationError,
    domains::invoices::entities::{Invoice, InvoiceFilter},
};

#[async_trait]
pub trait InvoicesUseCases: Send + Sync {
    async fn generate_invoice(
        &self,
        user_id: String,
        amount: u64,
        description: Option<String>,
        expiry: Option<u32>,
    ) -> Result<Invoice, ApplicationError>;
    async fn get_invoice(&self, id: Uuid) -> Result<Invoice, ApplicationError>;
    async fn list_invoices(&self, filter: InvoiceFilter) -> Result<Vec<Invoice>, ApplicationError>;
    async fn delete_invoice(&self, id: Uuid) -> Result<(), ApplicationError>;
    async fn delete_invoices(&self, filter: InvoiceFilter) -> Result<u64, ApplicationError>;
}
