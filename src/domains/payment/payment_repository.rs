use async_trait::async_trait;
use uuid::Uuid;

use crate::application::errors::DatabaseError;

use super::{Payment, PaymentFilter, PaymentStatus};

#[cfg_attr(test, mockall::automock)]
#[async_trait]
pub trait PaymentRepository: Send + Sync {
    async fn find(&self, id: Uuid) -> Result<Option<Payment>, DatabaseError>;
    async fn find_by_payment_hash(&self, payment_hash: &str) -> Result<Option<Payment>, DatabaseError>;
    async fn find_many(&self, filter: PaymentFilter) -> Result<Vec<Payment>, DatabaseError>;
    async fn insert(&self, payment: Payment) -> Result<Payment, DatabaseError>;
    async fn update(&self, payment: Payment) -> Result<Payment, DatabaseError>;
    /// Atomically move a payment from `from` to `to`, returning whether this
    /// call performed the transition. Makes finalization single-winner when the
    /// synchronous result and the success/failure event race for one payment.
    async fn try_transition(&self, id: Uuid, from: PaymentStatus, to: PaymentStatus) -> Result<bool, DatabaseError>;
    async fn delete_many(&self, filter: PaymentFilter) -> Result<u64, DatabaseError>;
    async fn max_btc_block_height(&self) -> Result<Option<u32>, DatabaseError>;
}
