use async_trait::async_trait;
use uuid::Uuid;

use crate::application::{entities::Currency, errors::DatabaseError};

#[cfg_attr(test, mockall::automock)]
#[async_trait]
pub trait WalletBalanceRepository: Send + Sync {
    /// Credit available balance for incoming funds, creating the wallet+currency row if absent.
    async fn credit(&self, wallet_id: Uuid, currency: &Currency, amount_msat: u64) -> Result<(), DatabaseError>;

    /// Move `amount_msat` from available to reserved. Returns `false` if available balance is insufficient.
    async fn reserve(&self, wallet_id: Uuid, currency: &Currency, amount_msat: u64) -> Result<bool, DatabaseError>;

    /// Debit available balance. Returns `false` if available balance is insufficient.
    async fn debit(&self, wallet_id: Uuid, currency: &Currency, amount_msat: u64) -> Result<bool, DatabaseError>;

    /// Move `amount_msat` from reserved back to available. Returns `false` if the reservation is missing.
    async fn release(&self, wallet_id: Uuid, currency: &Currency, amount_msat: u64) -> Result<bool, DatabaseError>;
}
