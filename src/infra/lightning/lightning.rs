use async_trait::async_trait;
use breez_sdk_core::{
    LnUrlPayRequestData, LspInformation, NodeState, Payment, ServiceHealthCheckResponse,
};

use crate::{
    application::errors::LightningError,
    domains::lightning::entities::{LightningInvoice, LightningPayment},
};

#[async_trait]
pub trait LightningClient: Sync + Send {
    fn node_info(&self) -> Result<NodeState, LightningError>;
    async fn lsp_info(&self) -> Result<LspInformation, LightningError>;
    async fn list_payments(&self) -> Result<Vec<Payment>, LightningError>;
    async fn invoice(
        &self,
        amount_msat: u64,
        description: String,
    ) -> Result<LightningInvoice, LightningError>;
    async fn payment_by_hash(
        &self,
        payment_hash: String,
    ) -> Result<Option<Payment>, LightningError>;
    async fn send_payment(
        &self,
        bolt11: String,
        amount_msat: Option<u64>,
    ) -> Result<LightningPayment, LightningError>;
    async fn send_spontaneous_payment(
        &self,
        node_id: String,
        amount_msat: u64,
    ) -> Result<LightningPayment, LightningError>;
    async fn lnurl_pay(
        &self,
        data: LnUrlPayRequestData,
        amount_msat: u64,
        comment: Option<String>,
    ) -> Result<LightningPayment, LightningError>;
    async fn health(&self) -> Result<ServiceHealthCheckResponse, LightningError>;
}
