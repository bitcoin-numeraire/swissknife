use async_trait::async_trait;

use crate::{application::errors::LightningError, domains::lightning::entities::LightningAddress};

#[async_trait]
pub trait LightningUseCases {
    async fn register_lightning_address(
        &self,
        username: String,
    ) -> Result<LightningAddress, LightningError>;
}
