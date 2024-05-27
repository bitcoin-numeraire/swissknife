use async_trait::async_trait;
use breez_sdk_core::{
    LspInformation, NodeState, Payment, ReverseSwapInfo, ServiceHealthCheckResponse,
};
use uuid::Uuid;

use crate::{
    application::{dtos::SendPaymentRequest, errors::ApplicationError},
    domains::{
        lightning::entities::{
            LNURLPayRequest, LightningAddress, LightningAddressFilter, LightningInvoice,
            LightningInvoiceFilter, LightningPayment, LightningPaymentFilter,
        },
        users::entities::AuthUser,
    },
};

#[async_trait]
pub trait LightningInvoicesUseCases {
    async fn generate_invoice(
        &self,
        user_id: String,
        amount: u64,
        description: Option<String>,
        expiry: Option<u32>,
    ) -> Result<LightningInvoice, ApplicationError>;
    async fn get_invoice(&self, id: Uuid) -> Result<LightningInvoice, ApplicationError>;
    async fn list_invoices(
        &self,
        filter: LightningInvoiceFilter,
    ) -> Result<Vec<LightningInvoice>, ApplicationError>;
    async fn delete_invoice(&self, id: Uuid) -> Result<(), ApplicationError>;
    async fn delete_invoices(
        &self,
        filter: LightningInvoiceFilter,
    ) -> Result<u64, ApplicationError>;
}

#[async_trait]
pub trait LightningPaymentsUseCases {
    async fn pay(&self, req: SendPaymentRequest) -> Result<LightningPayment, ApplicationError>;
    async fn get_payment(&self, id: Uuid) -> Result<LightningPayment, ApplicationError>;
    async fn list_payments(
        &self,
        filter: LightningPaymentFilter,
    ) -> Result<Vec<LightningPayment>, ApplicationError>;
    async fn delete_payment(&self, id: Uuid) -> Result<(), ApplicationError>;
    async fn delete_payments(
        &self,
        filter: LightningPaymentFilter,
    ) -> Result<u64, ApplicationError>;
}

#[async_trait]
pub trait LightningAddressesUseCases {
    async fn generate_lnurlp(&self, username: String) -> Result<LNURLPayRequest, ApplicationError>;
    async fn generate_lnurlp_invoice(
        &self,
        username: String,
        amount: u64,
        description: Option<String>,
    ) -> Result<LightningInvoice, ApplicationError>;

    async fn register_address(
        &self,
        user_id: String,
        username: String,
    ) -> Result<LightningAddress, ApplicationError>;
    async fn get_address(&self, id: Uuid) -> Result<LightningAddress, ApplicationError>;
    async fn list_addresses(
        &self,
        filter: LightningAddressFilter,
    ) -> Result<Vec<LightningAddress>, ApplicationError>;
    async fn delete_address(&self, id: Uuid) -> Result<(), ApplicationError>;
    async fn delete_addresses(
        &self,
        filter: LightningAddressFilter,
    ) -> Result<u64, ApplicationError>;
}

#[async_trait]
pub trait LightningNodeUseCases {
    async fn node_info(&self, user: AuthUser) -> Result<NodeState, ApplicationError>;
    async fn lsp_info(&self, user: AuthUser) -> Result<LspInformation, ApplicationError>;
    async fn list_lsps(&self, user: AuthUser) -> Result<Vec<LspInformation>, ApplicationError>;
    async fn list_node_payments(&self, user: AuthUser) -> Result<Vec<Payment>, ApplicationError>;
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

pub trait LightningUseCases:
    LightningPaymentsUseCases
    + LightningInvoicesUseCases
    + LightningAddressesUseCases
    + LightningNodeUseCases
    + Send
    + Sync
{
}

// Ensure that any type that implements the individual traits also implements the new trait.
impl<T> LightningUseCases for T where
    T: LightningPaymentsUseCases
        + LightningInvoicesUseCases
        + LightningAddressesUseCases
        + LightningNodeUseCases
        + Send
        + Sync
{
}
