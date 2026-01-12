use async_trait::async_trait;
use uuid::Uuid;

use crate::{application::errors::ApplicationError, domains::bitcoin::BitcoinAddressType};

use super::BitcoinAddress;

#[async_trait]
pub trait BitcoinUseCases: Send + Sync {
    async fn get_deposit_address(
        &self,
        wallet_id: Uuid,
        address_type: Option<BitcoinAddressType>,
    ) -> Result<BitcoinAddress, ApplicationError>;
    async fn sync_pending_transactions(&self) -> Result<(), ApplicationError>;
}
