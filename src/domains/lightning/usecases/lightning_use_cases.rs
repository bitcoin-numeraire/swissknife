use async_trait::async_trait;

use crate::{application::errors::LightningError, domains::lightning::entities::LightningAddress};

#[async_trait]
pub trait LightningUseCases: Send + Sync {
    async fn register_lightning_address(
        &self,
        user_id: String,
        username: String,
    ) -> Result<LightningAddress, LightningError>;
}
