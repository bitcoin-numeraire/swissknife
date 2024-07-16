use async_trait::async_trait;
use sea_orm::DatabaseTransaction;

use crate::application::errors::DatabaseError;

use super::UserBalance;

#[async_trait]
pub trait WalletRepository: Send + Sync {
    async fn get_balance(
        &self,
        txn: Option<&DatabaseTransaction>,
        user: &str,
    ) -> Result<UserBalance, DatabaseError>;
}
