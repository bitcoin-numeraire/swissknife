use std::sync::Arc;

use async_trait::async_trait;

use crate::application::errors::LightningError;

#[async_trait]
pub trait LightningClient {
    async fn invoice(
        &self,
        amount_msat: u64,
        description: String,
    ) -> Result<String, LightningError>;
}
pub type DynLightningClient = Arc<dyn LightningClient + Send + Sync>;
