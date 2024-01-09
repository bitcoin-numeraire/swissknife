use async_trait::async_trait;
use breez_sdk_core::{NodeState, Payment};

use crate::{
    application::errors::ApplicationError,
    domains::{
        lightning::entities::{LNURLp, LightningAddress},
        users::entities::AuthUser,
    },
};

#[async_trait]
pub trait LightningAddressesUseCases: Send + Sync {
    async fn generate_lnurlp(&self, username: String) -> Result<LNURLp, ApplicationError>;

    async fn generate_invoice(
        &self,
        username: String,
        amount: u64,
    ) -> Result<String, ApplicationError>;

    async fn register_lightning_address(
        &self,
        user: AuthUser,
        username: String,
    ) -> Result<LightningAddress, ApplicationError>;
}

#[async_trait]
pub trait LightningNodeUseCases: Send + Sync {
    async fn node_info(&self, user: AuthUser) -> Result<NodeState, ApplicationError>;
    async fn list_payments(&self, user: AuthUser) -> Result<Vec<Payment>, ApplicationError>;
}

pub trait LightningUseCases: LightningAddressesUseCases + LightningNodeUseCases {}

// Ensure that any type that implements the individual traits also implements the new trait.
impl<T> LightningUseCases for T where T: LightningAddressesUseCases + LightningNodeUseCases {}
