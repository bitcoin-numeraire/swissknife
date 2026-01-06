use async_trait::async_trait;
use uuid::Uuid;

use crate::application::errors::DatabaseError;

use super::{BitcoinAddress, BitcoinAddressType, BitcoinOutput};

#[async_trait]
pub trait BitcoinAddressRepository: Send + Sync {
    async fn find_by_wallet_unused(
        &self,
        wallet_id: Uuid,
        address_type: BitcoinAddressType,
    ) -> Result<Option<BitcoinAddress>, DatabaseError>;
    async fn find_by_address(&self, address: &str) -> Result<Option<BitcoinAddress>, DatabaseError>;
    async fn insert(&self, address: BitcoinAddress) -> Result<BitcoinAddress, DatabaseError>;
    async fn mark_used(&self, id: Uuid) -> Result<(), DatabaseError>;
}

#[async_trait]
pub trait BitcoinOutputRepository: Send + Sync {
    async fn find_by_outpoint(&self, outpoint: &str) -> Result<Option<BitcoinOutput>, DatabaseError>;
    async fn find(&self, id: Uuid) -> Result<Option<BitcoinOutput>, DatabaseError>;
    async fn upsert(&self, output: BitcoinOutput) -> Result<BitcoinOutput, DatabaseError>;
}
