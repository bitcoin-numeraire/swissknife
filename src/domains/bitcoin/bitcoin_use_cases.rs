use async_trait::async_trait;
use uuid::Uuid;

use crate::{
    application::errors::ApplicationError,
    domains::bitcoin::{BtcAddressFilter, BtcAddressType},
};

use super::BtcAddress;

#[async_trait]
pub trait BitcoinUseCases: Send + Sync {
    async fn new_deposit_address(
        &self,
        wallet_id: Uuid,
        address_type: Option<BtcAddressType>,
    ) -> Result<BtcAddress, ApplicationError>;
    async fn get_address(&self, id: Uuid) -> Result<BtcAddress, ApplicationError>;
    async fn list_addresses(&self, filter: BtcAddressFilter) -> Result<Vec<BtcAddress>, ApplicationError>;
    async fn delete_address(&self, id: Uuid) -> Result<(), ApplicationError>;
    async fn delete_many_addresses(&self, filter: BtcAddressFilter) -> Result<u64, ApplicationError>;
    async fn sync_pending_transactions(&self) -> Result<(), ApplicationError>;
}
