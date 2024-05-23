use async_trait::async_trait;
use breez_sdk_core::{
    LspInformation, NodeState, Payment, PaymentFailedData, ReverseSwapInfo,
    ServiceHealthCheckResponse,
};
use uuid::Uuid;

use crate::{
    application::errors::ApplicationError,
    domains::{
        lightning::entities::{
            LNURLPayRequest, LightningAddress, LightningInvoice, LightningPayment, UserBalance,
        },
        users::entities::AuthUser,
    },
};

// TODO: This trait is not necessarily linked to Lighting, move to a better domain once other use cases arise.
#[async_trait]
pub trait WalletUseCases {
    async fn get_balance(&self, user: AuthUser) -> Result<UserBalance, ApplicationError>;
}

#[async_trait]
pub trait LightningInvoicesUseCases {
    async fn generate_invoice(
        &self,
        user: AuthUser,
        amount: u64,
        description: Option<String>,
        expiry: Option<u32>,
    ) -> Result<LightningInvoice, ApplicationError>;

    async fn get_invoice(
        &self,
        user: AuthUser,
        id: Uuid,
    ) -> Result<LightningInvoice, ApplicationError>;

    async fn list_invoices(
        &self,
        user: AuthUser,
        limit: Option<u64>,
        offset: Option<u64>,
    ) -> Result<Vec<LightningInvoice>, ApplicationError>;
}

#[async_trait]
pub trait LightningPaymentsUseCases {
    async fn get_payment(
        &self,
        user: AuthUser,
        id: Uuid,
    ) -> Result<LightningPayment, ApplicationError>;

    async fn list_payments(
        &self,
        user: AuthUser,
        limit: Option<u64>,
        offset: Option<u64>,
    ) -> Result<Vec<LightningPayment>, ApplicationError>;

    async fn pay(
        &self,
        user: AuthUser,
        input: String,
        amount_msat: Option<u64>,
        comment: Option<String>,
    ) -> Result<LightningPayment, ApplicationError>;
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
        user: AuthUser,
        username: String,
    ) -> Result<LightningAddress, ApplicationError>;

    async fn get_address(
        &self,
        user: AuthUser,
        username: String,
    ) -> Result<LightningAddress, ApplicationError>;

    async fn list_addresses(
        &self,
        user: AuthUser,
        limit: Option<u64>,
        offset: Option<u64>,
    ) -> Result<Vec<LightningAddress>, ApplicationError>;
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

#[async_trait]
pub trait LightningPaymentsProcessorUseCases: Send + Sync {
    async fn process_incoming_payment(
        &self,
        payment: Payment,
    ) -> Result<LightningInvoice, ApplicationError>;
    async fn process_outgoing_payment(
        &self,
        payment: Payment,
    ) -> Result<LightningPayment, ApplicationError>;
    async fn process_failed_payment(
        &self,
        payment: PaymentFailedData,
    ) -> Result<LightningPayment, ApplicationError>;
}

pub trait LightningUseCases:
    WalletUseCases
    + LightningPaymentsUseCases
    + LightningInvoicesUseCases
    + LightningAddressesUseCases
    + LightningNodeUseCases
    + Send
    + Sync
{
}

// Ensure that any type that implements the individual traits also implements the new trait.
impl<T> LightningUseCases for T where
    T: WalletUseCases
        + LightningPaymentsUseCases
        + LightningInvoicesUseCases
        + LightningAddressesUseCases
        + LightningNodeUseCases
        + Send
        + Sync
{
}
