use async_trait::async_trait;
use sea_orm::DatabaseTransaction;
use uuid::Uuid;

use crate::{application::errors::DatabaseError, domains::wallet::Contact};

use super::{Payment, PaymentFilter};

#[async_trait]
pub trait PaymentRepository: Send + Sync {
    async fn find(&self, id: Uuid) -> Result<Option<Payment>, DatabaseError>;
    async fn find_by_payment_hash(
        &self,
        payment_hash: &str,
    ) -> Result<Option<Payment>, DatabaseError>;
    async fn find_many(&self, filter: PaymentFilter) -> Result<Vec<Payment>, DatabaseError>;
    async fn insert(
        &self,
        txn: Option<&DatabaseTransaction>,
        payment: Payment,
    ) -> Result<Payment, DatabaseError>;
    async fn update(&self, payment: Payment) -> Result<Payment, DatabaseError>;
    async fn delete_many(&self, filter: PaymentFilter) -> Result<u64, DatabaseError>;
    async fn find_contacts(&self, wallet_id: Uuid) -> Result<Vec<Contact>, DatabaseError>;
}
