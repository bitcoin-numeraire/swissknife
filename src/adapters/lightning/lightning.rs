use std::sync::Arc;

use async_trait::async_trait;

use crate::application::errors::ApplicationError;

#[async_trait]
pub trait LightningClient {
    async fn get_invoice(&self, amount: u64) -> Result<String, ApplicationError>;
}
pub type DynLightningClient = Arc<dyn LightningClient + Send + Sync>;
