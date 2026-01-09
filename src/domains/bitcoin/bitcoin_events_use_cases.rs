use async_trait::async_trait;

use crate::application::errors::ApplicationError;

use super::BitcoinTransaction;

#[async_trait]
pub trait BitcoinEventsUseCases: Send + Sync {
    async fn onchain_transaction(&self, transaction: BitcoinTransaction) -> Result<(), ApplicationError>;
    async fn sync_pending_transactions(&self) -> Result<(), ApplicationError>;
}
