use async_trait::async_trait;
use breez_sdk_core::{NodeState, Payment};

use crate::application::errors::LightningError;

#[async_trait]
pub trait LightningClient: Sync + Send {
    fn node_info(&self) -> Result<NodeState, LightningError>;
    async fn list_payments(&self) -> Result<Vec<Payment>, LightningError>;
    async fn invoice(
        &self,
        amount_msat: u64,
        description: String,
    ) -> Result<String, LightningError>;
}
