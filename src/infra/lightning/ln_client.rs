use async_trait::async_trait;
use breez_sdk_core::ReverseSwapInfo;
use chrono::{DateTime, Utc};

use crate::{
    application::{
        entities::Currency,
        errors::LightningError,
    },
    domains::{invoice::Invoice, payment::Payment, system::HealthStatus},
};

/// Bitcoin L1 transaction information returned by the Lightning provider
#[derive(Debug, Clone)]
pub struct BtcTransaction {
    pub txid: String,
    pub confirmed: bool,
    pub amount_sat: u64,
    pub fee_sat: Option<u64>,
    pub address: Option<String>,
    pub is_outgoing: bool,
    pub block_height: Option<u32>,
    pub timestamp: Option<DateTime<Utc>>,
}

/// Bitcoin L1 balance information
#[derive(Debug, Clone)]
pub struct BtcBalance {
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

    // Bitcoin L1 wallet methods (default implementations return NotSupported)
    async fn get_new_btc_address(&self) -> Result<String, LightningError> {
        Err(LightningError::NotSupported(
            "Bitcoin address generation".to_string(),
        ))
    }

    async fn get_btc_balance(&self) -> Result<BtcBalance, LightningError> {
        Err(LightningError::NotSupported(
            "Bitcoin balance".to_string(),
        ))
    }

    async fn send_btc(
        &self,
        _address: &str,
        _amount_sat: u64,
        _fee_rate_sat_per_vbyte: Option<u32>,
    ) -> Result<String, LightningError> {
        Err(LightningError::NotSupported(
            "Bitcoin send".to_string(),
        ))
    }

    async fn list_btc_transactions(&self) -> Result<Vec<BtcTransaction>, LightningError> {
        Err(LightningError::NotSupported(
            "Bitcoin transaction listing".to_string(),
        ))
    }

    async fn get_network(&self) -> Result<Currency, LightningError> {
        Err(LightningError::NotSupported(
            "Network info".to_string(),
        ))
    }

    async fn validate_btc_address(&self, _address: &str) -> Result<bool, LightningError> {
        Err(LightningError::NotSupported(
            "Bitcoin address validation".to_string(),
        ))
    }
}
