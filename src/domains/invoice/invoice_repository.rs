use async_trait::async_trait;
use sea_orm::DatabaseTransaction;
use uuid::Uuid;

use crate::application::errors::DatabaseError;

use super::{Invoice, InvoiceFilter};

#[async_trait]
pub trait InvoiceRepository: Send + Sync {
    async fn find(&self, id: Uuid) -> Result<Option<Invoice>, DatabaseError>;
    async fn find_by_payment_hash(
        &self,
        payment_hash: &str,
    ) -> Result<Option<Invoice>, DatabaseError>;
    async fn find_many(&self, filter: InvoiceFilter) -> Result<Vec<Invoice>, DatabaseError>;
    async fn insert(
        &self,
        txn: Option<&DatabaseTransaction>,
        invoice: Invoice,
    ) -> Result<Invoice, DatabaseError>;
    async fn update(
        &self,
        txn: Option<&DatabaseTransaction>,
        invoice: Invoice,
    ) -> Result<Invoice, DatabaseError>;
    async fn delete_many(&self, filter: InvoiceFilter) -> Result<u64, DatabaseError>;
}
