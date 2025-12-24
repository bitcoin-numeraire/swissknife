use async_trait::async_trait;
use breez_sdk_core::ReverseSwapInfo;

use crate::{
    application::{entities::Currency, errors::LightningError},
    domains::{
        bitcoin::{BitcoinBalance, BitcoinTransaction},
        invoice::Invoice,
        payment::Payment,
        system::HealthStatus,
    },
};

#[allow(dead_code)]
#[async_trait]
pub trait LnClient: Sync + Send {
    async fn disconnect(&self) -> Result<(), LightningError>;
    async fn invoice(
        &self,
        amount_msat: u64,
        description: String,
        expiry: u32,
        deschashonly: bool,
    ) -> Result<Invoice, LightningError>;
    async fn pay(&self, bolt11: String, amount_msat: Option<u64>) -> Result<Payment, LightningError>;
    async fn pay_onchain(
        &self,
        amount_sat: u64,
        recipient_address: String,
        feerate: u32,
    ) -> Result<ReverseSwapInfo, LightningError>;
    async fn invoice_by_hash(&self, payment_hash: String) -> Result<Option<Invoice>, LightningError>;
    async fn health(&self) -> Result<HealthStatus, LightningError>;

    async fn get_new_bitcoin_address(&self) -> Result<String, LightningError>;
    async fn get_bitcoin_balance(&self) -> Result<BitcoinBalance, LightningError>;
    async fn send_bitcoin(
        &self,
        address: String,
        amount_sat: u64,
        fee_rate: Option<u32>,
    ) -> Result<String, LightningError>;
    async fn list_bitcoin_transactions(&self) -> Result<Vec<BitcoinTransaction>, LightningError>;
    async fn get_bitcoin_network(&self) -> Result<Currency, LightningError>;
    async fn validate_bitcoin_address(&self, address: &str) -> Result<bool, LightningError>;
}
