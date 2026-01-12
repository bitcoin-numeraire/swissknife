use async_trait::async_trait;

use crate::{application::errors::ApplicationError, domains::bitcoin::BitcoinNetwork};

use super::BitcoinTransaction;

#[async_trait]
pub trait BitcoinEventsUseCases: Send + Sync {
    async fn onchain_transaction(
        &self,
        transaction: BitcoinTransaction,
        network: BitcoinNetwork,
    ) -> Result<(), ApplicationError>;
}
