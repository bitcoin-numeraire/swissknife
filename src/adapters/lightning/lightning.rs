use std::sync::Arc;

use async_trait::async_trait;

use crate::{
    application::errors::ApplicationError, domains::lightning::entities::LightningInvoice,
};

#[async_trait]
pub trait LightningClient {
    async fn invoice(&self, amount: u64) -> Result<LightningInvoice, ApplicationError>;
}
pub type DynLightningClient = Arc<dyn LightningClient + Send + Sync>;
