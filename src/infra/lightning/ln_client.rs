use async_trait::async_trait;
use breez_sdk_core::ReverseSwapInfo;

use crate::{
    application::{
        entities::Currency,
        errors::LightningError,
    },
    domains::{invoice::Invoice, payment::Payment, system::HealthStatus},
};

/// Onchain transaction information returned by the Lightning provider
#[derive(Debug, Clone)]
pub struct OnchainTransaction {
    pub txid: String,
    pub confirmations: u32,
    pub amount_sat: u64,
    pub fee_sat: Option<u64>,
    pub address: Option<String>,
    pub is_outgoing: bool,
    pub block_height: Option<u32>,
}

/// Onchain balance information
#[derive(Debug, Clone)]
pub struct OnchainBalance {
    pub confirmed_sat: u64,
    pub unconfirmed_sat: u64,
}

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

    // Onchain wallet methods
    async fn get_new_onchain_address(&self) -> Result<String, LightningError>;
    async fn get_onchain_balance(&self) -> Result<OnchainBalance, LightningError>;
    async fn send_onchain(
        &self,
        address: String,
        amount_sat: u64,
        fee_rate_sat_per_vbyte: Option<u32>,
    ) -> Result<String, LightningError>; // Returns txid
    async fn list_onchain_transactions(&self) -> Result<Vec<OnchainTransaction>, LightningError>;
    async fn get_network(&self) -> Result<Currency, LightningError>;
    async fn validate_address(&self, address: &str) -> Result<bool, LightningError>;
}
