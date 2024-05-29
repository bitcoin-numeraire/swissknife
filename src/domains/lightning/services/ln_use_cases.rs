use async_trait::async_trait;
use breez_sdk_core::{
    LspInformation, NodeState, Payment as BreezPayment, ReverseSwapInfo, ServiceHealthCheckResponse,
};
use uuid::Uuid;

use crate::{
    application::errors::ApplicationError,
    domains::{
        invoices::entities::Invoice,
        lightning::entities::{LNURLPayRequest, LnAddress, LnAddressFilter},
        users::entities::AuthUser,
    },
};

#[async_trait]
pub trait LnAddressesUseCases: Send + Sync {
    async fn generate_lnurlp(&self, username: String) -> Result<LNURLPayRequest, ApplicationError>;
    async fn generate_lnurlp_invoice(
        &self,
        username: String,
        amount: u64,
        description: Option<String>,
    ) -> Result<Invoice, ApplicationError>;
    async fn register(
        &self,
        user_id: String,
        username: String,
    ) -> Result<LnAddress, ApplicationError>;
    async fn get(&self, id: Uuid) -> Result<LnAddress, ApplicationError>;
    async fn list(&self, filter: LnAddressFilter) -> Result<Vec<LnAddress>, ApplicationError>;
    async fn delete(&self, id: Uuid) -> Result<(), ApplicationError>;
    async fn delete_many(&self, filter: LnAddressFilter) -> Result<u64, ApplicationError>;
}

#[async_trait]
pub trait LnNodeUseCases: Send + Sync {
    async fn node_info(&self, user: AuthUser) -> Result<NodeState, ApplicationError>;
    async fn lsp_info(&self, user: AuthUser) -> Result<LspInformation, ApplicationError>;
    async fn list_lsps(&self, user: AuthUser) -> Result<Vec<LspInformation>, ApplicationError>;
    async fn list_node_payments(
        &self,
        user: AuthUser,
    ) -> Result<Vec<BreezPayment>, ApplicationError>;
    async fn close_lsp_channels(&self, user: AuthUser) -> Result<Vec<String>, ApplicationError>;
    async fn pay_onchain(
        &self,
        user: AuthUser,
        amount_sat: u64,
        recipient_address: String,
        feerate: u32,
    ) -> Result<ReverseSwapInfo, ApplicationError>;
    async fn health_check(
        &self,
        user: AuthUser,
    ) -> Result<ServiceHealthCheckResponse, ApplicationError>;
    async fn redeem(
        &self,
        user: AuthUser,
        to_address: String,
        feerate: u32,
    ) -> Result<String, ApplicationError>;
}
