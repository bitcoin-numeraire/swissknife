use async_trait::async_trait;
use breez_sdk_core::{
    LnUrlPayRequestData, LspInformation, NodeState, Payment as BreezPayment, ReverseSwapInfo,
    ServiceHealthCheckResponse,
};
use uuid::Uuid;

use crate::{
    application::errors::LightningError,
    domains::{invoices::entities::Invoice, payments::entities::Payment},
};

#[async_trait]
pub trait LnClient: Sync + Send {
    fn node_info(&self) -> Result<NodeState, LightningError>;
    async fn lsp_info(&self) -> Result<LspInformation, LightningError>;
    async fn list_payments(&self) -> Result<Vec<BreezPayment>, LightningError>;
    async fn invoice(
        &self,
        amount_msat: u64,
        description: String,
        expiry: u32,
    ) -> Result<Invoice, LightningError>;
    async fn payment_by_hash(
        &self,
        payment_hash: String,
    ) -> Result<Option<BreezPayment>, LightningError>;
    async fn send_payment(
        &self,
        bolt11: String,
        amount_msat: Option<u64>,
        label: Uuid,
    ) -> Result<Payment, LightningError>;
    async fn send_spontaneous_payment(
        &self,
        node_id: String,
        amount_msat: u64,
        label: Uuid,
    ) -> Result<Payment, LightningError>;
    async fn lnurl_pay(
        &self,
        data: LnUrlPayRequestData,
        amount_msat: u64,
        comment: Option<String>,
        label: Uuid,
    ) -> Result<Payment, LightningError>;
    async fn health(&self) -> Result<ServiceHealthCheckResponse, LightningError>;
    async fn list_lsps(&self) -> Result<Vec<LspInformation>, LightningError>;
    async fn close_lsp_channels(&self) -> Result<Vec<String>, LightningError>;
    async fn pay_onchain(
        &self,
        amount_sat: u64,
        recipient_address: String,
        feerate: u32,
    ) -> Result<ReverseSwapInfo, LightningError>;
    async fn redeem_onchain(
        &self,
        to_address: String,
        feerate: u32,
    ) -> Result<String, LightningError>;
}
