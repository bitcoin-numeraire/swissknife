use async_trait::async_trait;
use breez_sdk_core::{
    LspInformation, NodeState, Payment as BreezPayment, PaymentFailedData, ReverseSwapInfo,
    ServiceHealthCheckResponse,
};
use uuid::Uuid;

use crate::{
    application::errors::ApplicationError,
    domains::{
        invoices::entities::Invoice,
        lightning::entities::{LnAddress, LnAddressFilter, LnInvoicePaidEvent, LnURLPayRequest},
        users::entities::AuthUser,
    },
};

#[async_trait]
pub trait LnAddressesUseCases: Send + Sync {
    async fn lnurlp(&self, username: String) -> Result<LnURLPayRequest, ApplicationError>;
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
    async fn info(&self, user: AuthUser) -> Result<NodeState, ApplicationError>;
    async fn lsp(&self, user: AuthUser) -> Result<LspInformation, ApplicationError>;
    async fn list_lsps(&self, user: AuthUser) -> Result<Vec<LspInformation>, ApplicationError>;
    async fn list_payments(&self, user: AuthUser) -> Result<Vec<BreezPayment>, ApplicationError>;
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

#[async_trait]
pub trait LnEventsUseCases: Send + Sync {
    async fn latest_settled_invoice(&self) -> Result<Option<Invoice>, ApplicationError>;
    async fn invoice_paid(&self, event: LnInvoicePaidEvent) -> Result<(), ApplicationError>;
    async fn outgoing_payment(&self, payment: BreezPayment) -> Result<(), ApplicationError>;
    async fn failed_payment(&self, payment: PaymentFailedData) -> Result<(), ApplicationError>;
}
