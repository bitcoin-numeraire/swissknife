use async_trait::async_trait;
use chrono::{DateTime, Utc};

use crate::{
    application::errors::ApplicationError,
    domains::{bitcoin::BtcNetwork, invoice::Invoice},
};

#[derive(Debug, Clone)]
pub struct LnInvoicePaidEvent {
    pub payment_hash: String,
    pub amount_received_msat: u64,
    pub fee_msat: u64,
    pub payment_time: DateTime<Utc>,
}

#[derive(Debug, Clone)]
pub struct LnPaySuccessEvent {
    pub amount_msat: u64,
    pub fees_msat: u64,
    pub payment_hash: String,
    pub payment_preimage: String,
    pub payment_time: DateTime<Utc>,
}

#[derive(Debug, Clone)]
pub struct LnPayFailureEvent {
    pub reason: String,
    pub payment_hash: String,
}

#[derive(Debug, Clone)]
pub struct BtcOutputEvent {
    pub txid: String,
    pub output_index: u32,
    pub address: Option<String>,
    pub amount_sat: u64,
    pub timestamp: DateTime<Utc>,
    pub fee_sat: Option<u64>,
    pub block_height: u32,
    pub network: BtcNetwork,
}

#[async_trait]
pub trait EventsUseCases: Send + Sync {
    async fn latest_settled_invoice(&self) -> Result<Option<Invoice>, ApplicationError>;
    async fn invoice_paid(&self, event: LnInvoicePaidEvent) -> Result<(), ApplicationError>;
    async fn outgoing_payment(&self, event: LnPaySuccessEvent) -> Result<(), ApplicationError>;
    async fn failed_payment(&self, event: LnPayFailureEvent) -> Result<(), ApplicationError>;
    async fn onchain_deposit(&self, output: BtcOutputEvent) -> Result<(), ApplicationError>;
    async fn onchain_withdrawal(&self, output: BtcOutputEvent) -> Result<(), ApplicationError>;
}
