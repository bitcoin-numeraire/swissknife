use async_trait::async_trait;
use uuid::Uuid;

use crate::application::errors::DatabaseError;

use super::{Balance, Contact, Wallet, WalletFilter, WalletOverview};

#[cfg_attr(test, mockall::automock)]
#[async_trait]
pub trait WalletRepository: Send + Sync {
    async fn find(&self, id: Uuid) -> Result<Option<Wallet>, DatabaseError>;
    async fn exists_for_account(&self, account_id: Uuid, id: Uuid) -> Result<bool, DatabaseError>;
    async fn find_by_account_and_asset(
        &self,
        account_id: Uuid,
        asset_id: Uuid,
    ) -> Result<Option<Wallet>, DatabaseError>;
    async fn find_many(&self, filter: WalletFilter) -> Result<Vec<Wallet>, DatabaseError>;
    async fn find_many_overview(&self) -> Result<Vec<WalletOverview>, DatabaseError>;
    async fn upsert(&self, account_id: Uuid, asset_id: Uuid) -> Result<Wallet, DatabaseError>;
    async fn get_balance(&self, id: Uuid) -> Result<Balance, DatabaseError>;
    /// Credit available balance for incoming funds.
    async fn credit(&self, id: Uuid, amount_msat: u64) -> Result<(), DatabaseError>;
    /// Move `amount_msat` from available to reserved. Returns `false` if available balance is insufficient.
    async fn reserve(&self, id: Uuid, amount_msat: u64) -> Result<bool, DatabaseError>;
    /// Debit available balance. Returns `false` if available balance is insufficient.
    async fn debit(&self, id: Uuid, amount_msat: u64) -> Result<bool, DatabaseError>;
    /// Debit a confirmed external spend, allowing available balance to go negative.
    async fn debit_confirmed(&self, id: Uuid, amount_msat: u64) -> Result<bool, DatabaseError>;
    /// Move `amount_msat` from reserved back to available. Returns `false` if the reservation is missing.
    async fn release(&self, id: Uuid, amount_msat: u64) -> Result<bool, DatabaseError>;
    async fn find_contacts(&self, id: Uuid) -> Result<Vec<Contact>, DatabaseError>;
    async fn delete_many(&self, filter: WalletFilter) -> Result<u64, DatabaseError>;
}
