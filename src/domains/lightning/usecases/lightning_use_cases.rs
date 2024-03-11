use async_trait::async_trait;
use breez_sdk_core::{LspInformation, NodeState, Payment};

use crate::{
    application::errors::ApplicationError,
    domains::{
        lightning::entities::{
            LNURLPayRequest, LightningAddress, LightningInvoice, LightningPayment,
        },
        users::entities::AuthUser,
    },
};

#[async_trait]
pub trait LightningAddressesUseCases: Send + Sync {
    async fn generate_lnurlp(&self, username: String) -> Result<LNURLPayRequest, ApplicationError>;

    async fn generate_invoice(
        &self,
        username: String,
        amount: u64,
        description: String,
    ) -> Result<LightningInvoice, ApplicationError>;

    async fn register_lightning_address(
        &self,
        user: AuthUser,
        username: String,
    ) -> Result<LightningAddress, ApplicationError>;

    async fn get_lightning_address(
        &self,
        user: AuthUser,
        username: String,
    ) -> Result<LightningAddress, ApplicationError>;

    async fn list_lightning_addresses(
        &self,
        user: AuthUser,
        limit: usize,
        offset: usize,
    ) -> Result<Vec<LightningAddress>, ApplicationError>;

    async fn send_payment(
        &self,
        user: AuthUser,
        input: String,
        amount_msat: Option<u64>,
        comment: Option<String>,
    ) -> Result<LightningPayment, ApplicationError>;
}

#[async_trait]
pub trait LightningPaymentsUseCases: Send + Sync {
    async fn process_incoming_payment(
        &self,
        payment: Payment,
    ) -> Result<LightningInvoice, ApplicationError>;
}

#[async_trait]
pub trait LightningNodeUseCases: Send + Sync {
    async fn node_info(&self, user: AuthUser) -> Result<NodeState, ApplicationError>;
    async fn lsp_info(&self, user: AuthUser) -> Result<LspInformation, ApplicationError>;
    async fn list_payments(&self, user: AuthUser) -> Result<Vec<Payment>, ApplicationError>;
    async fn send_bolt11_payment(
        &self,
        user: AuthUser,
        bolt11_invoice: String,
        amount_msat: Option<u64>,
    ) -> Result<LightningPayment, ApplicationError>;
}

pub trait LightningUseCases: LightningAddressesUseCases + LightningNodeUseCases {}

// Ensure that any type that implements the individual traits also implements the new trait.
impl<T> LightningUseCases for T where T: LightningAddressesUseCases + LightningNodeUseCases {}
