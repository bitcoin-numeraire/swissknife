use async_trait::async_trait;
use breez_sdk_core::ReverseSwapInfo;

use crate::{
    application::{entities::BitcoinWallet, errors::LightningError},
    domains::{invoice::Invoice, payment::Payment, system::HealthStatus},
};

#[async_trait]
pub trait LnClient: BitcoinWallet + Sync + Send {
    async fn disconnect(&self) -> Result<(), LightningError>;
    async fn pay_onchain(
        &self,
        amount_sat: u64,
        recipient_address: String,
        feerate: u32,
    ) -> Result<ReverseSwapInfo, LightningError>;
    async fn invoice(
        &self,
        amount_msat: u64,
        description: String,
        expiry: u32,
        deschashonly: bool,
    ) -> Result<Invoice, LightningError>;
    async fn pay(&self, bolt11: String, amount_msat: Option<u64>) -> Result<Payment, LightningError>;
    async fn invoice_by_hash(&self, payment_hash: String) -> Result<Option<Invoice>, LightningError>;
    async fn health(&self) -> Result<HealthStatus, LightningError>;
}
