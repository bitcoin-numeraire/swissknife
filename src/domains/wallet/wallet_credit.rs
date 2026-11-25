use async_trait::async_trait;
use swissknife_types::{CreateWalletCredit, WalletCreditReceipt};
use uuid::Uuid;

use crate::{application::errors::ApplicationError, domains::account::User};

#[cfg_attr(test, mockall::automock)]
#[async_trait]
pub trait WalletCreditUnitOfWork: Send + Sync {
    /// Persist an immutable receipt, credit the wallet, create its settled incoming
    /// activity, and append the client event as one idempotent transaction.
    async fn post(
        &self,
        wallet_id: Uuid,
        actor: &User,
        request: CreateWalletCredit,
    ) -> Result<WalletCreditReceipt, ApplicationError>;
}
