use async_trait::async_trait;
use breez_sdk_core::{LspInformation, NodeState, Payment};

use crate::application::errors::LightningError;

#[async_trait]
pub trait LightningClient: Sync + Send {
    fn node_info(&self) -> Result<NodeState, LightningError>;
    async fn lsp_info(&self) -> Result<LspInformation, LightningError>;
    async fn list_payments(&self) -> Result<Vec<Payment>, LightningError>;
    async fn invoice(
        &self,
        amount_msat: u64,
        description: String,
    ) -> Result<String, LightningError>;
    async fn send_payment(
        &self,
        bolt11: String,
        amount_msat: Option<u64>,
    ) -> Result<Payment, LightningError>;
}
