use async_trait::async_trait;
use uuid::Uuid;

use crate::application::errors::ApplicationError;

use super::{BitcoinAddress, BitcoinOutput};

#[async_trait]
pub trait BitcoinUseCases: Send + Sync {
    async fn get_deposit_address(&self, wallet_id: Uuid) -> Result<BitcoinAddress, ApplicationError>;
    async fn sync_outputs(&self) -> Result<Vec<BitcoinOutput>, ApplicationError>;
}
