use async_trait::async_trait;
use sea_orm::DatabaseTransaction;
use uuid::Uuid;

use crate::{
    application::errors::DatabaseError,
    domains::invoices::entities::{Invoice, InvoiceFilter},
};

#[async_trait]
pub trait InvoiceRepository: Send + Sync {
    async fn find_invoice(&self, id: Uuid) -> Result<Option<Invoice>, DatabaseError>;
    async fn find_invoice_by_payment_hash(
        &self,
        payment_hash: &str,
    ) -> Result<Option<Invoice>, DatabaseError>;
    async fn find_invoices(&self, filter: InvoiceFilter) -> Result<Vec<Invoice>, DatabaseError>;
    async fn insert_invoice(
        &self,
        txn: Option<&DatabaseTransaction>,
        invoice: Invoice,
    ) -> Result<Invoice, DatabaseError>;
    async fn update_invoice(
        &self,
        txn: Option<&DatabaseTransaction>,
        invoice: Invoice,
    ) -> Result<Invoice, DatabaseError>;
    async fn delete_invoices(&self, filter: InvoiceFilter) -> Result<u64, DatabaseError>;
}
