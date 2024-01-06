use async_trait::async_trait;
use breez_sdk_core::{NodeState, Payment};

use crate::{
    application::errors::LightningError,
    domains::lightning::entities::{LNURLp, LightningAddress},
};

#[async_trait]
pub trait LightningAddressesUseCases: Send + Sync {
    async fn generate_lnurlp(&self, username: String) -> Result<LNURLp, LightningError>;

    async fn generate_invoice(
        &self,
        username: String,
        amount: u64,
    ) -> Result<String, LightningError>;

    async fn register_lightning_address(
        &self,
        user_id: String,
        username: String,
    ) -> Result<LightningAddress, LightningError>;
}

#[async_trait]
pub trait LightningNodeUseCases: Send + Sync {
    async fn node_info(&self, username: String) -> Result<NodeState, LightningError>;
    async fn list_payments(&self, username: String) -> Result<Vec<Payment>, LightningError>;
}

pub trait LightningUseCases: LightningAddressesUseCases + LightningNodeUseCases {}

// Ensure that any type that implements the individual traits also implements the new trait.
impl<T> LightningUseCases for T where T: LightningAddressesUseCases + LightningNodeUseCases {}
