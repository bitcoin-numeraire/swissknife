use async_trait::async_trait;
use std::sync::Arc;

use super::{Asset, AssetRepository, AssetUseCases};
use crate::application::errors::ApplicationError;

pub struct AssetService {
    assets: Arc<dyn AssetRepository>,
}

impl AssetService {
    pub fn new(assets: Arc<dyn AssetRepository>) -> Self {
        Self { assets }
    }
}

#[async_trait]
impl AssetUseCases for AssetService {
    async fn list(&self) -> Result<Vec<Asset>, ApplicationError> {
        Ok(self.assets.list().await?)
    }
}
