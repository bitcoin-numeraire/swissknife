use async_trait::async_trait;
use uuid::Uuid;

use crate::{application::errors::DatabaseError, domains::bitcoin::BtcAddressFilter};

use super::{BtcAddress, BtcAddressType, BtcOutput};

#[async_trait]
pub trait BtcAddressRepository: Send + Sync {
    async fn find(&self, id: Uuid) -> Result<Option<BtcAddress>, DatabaseError>;
    async fn find_by_wallet_unused(
        &self,
        wallet_id: Uuid,
        address_type: BtcAddressType,
    ) -> Result<Option<BtcAddress>, DatabaseError>;
    async fn find_by_address(&self, address: &str) -> Result<Option<BtcAddress>, DatabaseError>;
    async fn find_many(&self, filter: BtcAddressFilter) -> Result<Vec<BtcAddress>, DatabaseError>;
    async fn insert(
        &self,
        wallet_id: Uuid,
        address: &str,
        address_type: BtcAddressType,
    ) -> Result<BtcAddress, DatabaseError>;
    async fn mark_used(&self, id: Uuid) -> Result<(), DatabaseError>;
    async fn delete_many(&self, filter: BtcAddressFilter) -> Result<u64, DatabaseError>;
}

#[async_trait]
pub trait BtcOutputRepository: Send + Sync {
    async fn find_by_outpoint(&self, outpoint: &str) -> Result<Option<BtcOutput>, DatabaseError>;
    async fn upsert(&self, output: BtcOutput) -> Result<BtcOutput, DatabaseError>;
}
