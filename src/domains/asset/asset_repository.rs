use async_trait::async_trait;

use crate::{
    application::errors::DatabaseError,
    domains::{asset::Asset, bitcoin::BtcNetwork},
};

#[cfg_attr(test, mockall::automock)]
#[async_trait]
pub trait AssetRepository: Send + Sync {
    async fn find_native_btc_by_network(&self, network: BtcNetwork) -> Result<Option<Asset>, DatabaseError>;
}
