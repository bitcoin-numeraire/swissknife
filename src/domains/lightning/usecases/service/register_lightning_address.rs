use async_trait::async_trait;

use crate::{
    application::errors::LightningError,
    domains::lightning::{entities::LightningAddress, usecases::LightningUseCases},
};

use super::LightningService;

#[async_trait]
impl LightningUseCases for LightningService {
    async fn register_lightning_address(
        &self,
        username: String,
    ) -> Result<LightningAddress, LightningError> {
        unimplemented!()
    }
}
