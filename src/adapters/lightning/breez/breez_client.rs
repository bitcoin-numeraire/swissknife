use async_trait::async_trait;

use crate::{
    adapters::lightning::LightningClient, application::errors::ApplicationError,
    domains::lightning::entities::LightningInvoice,
};

#[derive(Clone)]
pub struct BreezClientConfig {}

pub struct BreezClient {}

impl BreezClient {
    pub async fn new(config: BreezClientConfig) -> Result<Self, ApplicationError> {
        Ok(Self {})
    }
}

#[async_trait]
impl LightningClient for BreezClient {
    async fn invoice(&self, amount: u64) -> Result<LightningInvoice, ApplicationError> {
        unimplemented!()
    }
}
