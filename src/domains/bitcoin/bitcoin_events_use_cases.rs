use async_trait::async_trait;

use crate::application::errors::ApplicationError;

use super::BitcoinOutputEvent;

#[async_trait]
pub trait BitcoinEventsUseCases: Send + Sync {
    async fn onchain_deposit(&self, output: BitcoinOutputEvent) -> Result<(), ApplicationError>;
    async fn onchain_withdrawal(&self, output: BitcoinOutputEvent) -> Result<(), ApplicationError>;
}
