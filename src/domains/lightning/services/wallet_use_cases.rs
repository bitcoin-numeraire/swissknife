use async_trait::async_trait;
use uuid::Uuid;

use crate::{
    application::errors::ApplicationError,
    domains::{
        lightning::entities::{LightningAddress, LightningInvoice, UserBalance},
        users::entities::AuthUser,
    },
};

#[async_trait]
pub trait WalletUseCases: Send + Sync {
    async fn get_balance(&self, user: AuthUser) -> Result<UserBalance, ApplicationError>;
    async fn get_lightning_address(
        &self,
        user: AuthUser,
    ) -> Result<LightningAddress, ApplicationError>;
    async fn register_lightning_address(
        &self,
        user: AuthUser,
        username: String,
    ) -> Result<LightningAddress, ApplicationError>;
    async fn generate_Lightning_invoice(
        &self,
        user: AuthUser,
        amount: u64,
        description: Option<String>,
        expiry: Option<u32>,
    ) -> Result<LightningInvoice, ApplicationError>;
    async fn get_lightning_invoice(
        &self,
        user: AuthUser,
        id: Uuid,
    ) -> Result<LightningInvoice, ApplicationError>;
    async fn list_lightning_invoices(
        &self,
        user: AuthUser,
        limit: Option<u64>,
        offset: Option<u64>,
    ) -> Result<Vec<LightningInvoice>, ApplicationError>;
    async fn delete_expired_invoices(&self, user: AuthUser) -> Result<u64, ApplicationError>;
}
