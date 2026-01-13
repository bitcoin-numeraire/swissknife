use async_trait::async_trait;

use crate::{application::errors::ApplicationError, domains::bitcoin::BitcoinNetwork};

use super::BitcoinOutputEvent;

#[async_trait]
pub trait BitcoinEventsUseCases: Send + Sync {
    async fn onchain_deposit(
        &self,
        output: BitcoinOutputEvent,
        network: BitcoinNetwork,
    ) -> Result<(), ApplicationError>;
    async fn onchain_withdrawal(
        &self,
        output: BitcoinOutputEvent,
        network: BitcoinNetwork,
    ) -> Result<(), ApplicationError>;
}
