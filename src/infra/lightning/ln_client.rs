use async_trait::async_trait;
use breez_sdk_core::ReverseSwapInfo;

use crate::{
    application::errors::LightningError,
    domains::{
        invoices::entities::Invoice, payments::entities::Payment, system::entities::HealthStatus,
    },
};

#[async_trait]
pub trait LnClient: Sync + Send {
    async fn disconnect(&self) -> Result<(), LightningError>;
    async fn invoice(
        &self,
        amount_msat: u64,
        description: String,
        expiry: u32,
    ) -> Result<Invoice, LightningError>;
    async fn pay(
        &self,
        bolt11: String,
        amount_msat: Option<u64>,
    ) -> Result<Payment, LightningError>;
    async fn pay_onchain(
        &self,
        amount_sat: u64,
        recipient_address: String,
        feerate: u32,
    ) -> Result<ReverseSwapInfo, LightningError>;
    async fn invoice_by_hash(
        &self,
        payment_hash: String,
    ) -> Result<Option<Invoice>, LightningError>;
    async fn health(&self) -> Result<HealthStatus, LightningError>;
}
