use async_trait::async_trait;
use swissknife_types::{SetWalletIdentity, WalletIdentity};
use uuid::Uuid;

use crate::application::errors::ApplicationError;

use super::{Balance, Contact, Wallet, WalletFilter, WalletOverview};

#[cfg_attr(test, mockall::automock)]
#[async_trait]
pub trait WalletUseCases: Send + Sync {
    async fn create(&self, account_id: Uuid, asset_id: Uuid) -> Result<Wallet, ApplicationError>;
    async fn get(&self, id: Uuid) -> Result<Wallet, ApplicationError>;
    async fn get_by_account_id(&self, account_id: Uuid, id: Uuid) -> Result<Wallet, ApplicationError>;
    async fn verify_ownership(&self, account_id: Uuid, id: Uuid) -> Result<(), ApplicationError>;
    async fn list(&self, filter: WalletFilter) -> Result<Vec<Wallet>, ApplicationError>;
    async fn list_overviews(&self) -> Result<Vec<WalletOverview>, ApplicationError>;
    async fn identity(&self, account: Uuid, wallet: Uuid) -> Result<Option<WalletIdentity>, ApplicationError>;
    async fn set_identity(
        &self,
        account: Uuid,
        wallet: Uuid,
        request: SetWalletIdentity,
    ) -> Result<WalletIdentity, ApplicationError>;
    async fn get_balance(&self, id: Uuid) -> Result<Balance, ApplicationError>;
    async fn list_contacts(&self, id: Uuid) -> Result<Vec<Contact>, ApplicationError>;
    async fn delete(&self, id: Uuid) -> Result<(), ApplicationError>;
    async fn delete_many(&self, filter: WalletFilter) -> Result<u64, ApplicationError>;
}
