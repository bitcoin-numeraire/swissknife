use std::sync::Arc;

use async_trait::async_trait;
use rgb_lib::wallet::{Assets, Balance, Metadata, ReceiveData, Recipient, Unspent};

use crate::{application::errors::ApplicationError, domains::rgb::entities::RGBContract};

#[async_trait]
pub trait RGBClient {
    async fn get_address(&self) -> Result<String, ApplicationError>;
    async fn get_btc_balance(&self) -> Result<u64, ApplicationError>;
    async fn list_unspents(&self) -> Result<Vec<Unspent>, ApplicationError>;
    async fn send_btc(
        &self,
        address: String,
        amount: u64,
        fee_rate: f32,
    ) -> Result<String, ApplicationError>;
    async fn drain_btc(&self, address: String, fee_rate: f32) -> Result<String, ApplicationError>;
    async fn create_utxos(&self, fee_rate: f32) -> Result<u8, ApplicationError>;
    async fn issue_contract(&self, contract: RGBContract) -> Result<String, ApplicationError>;
    async fn list_assets(&self) -> Result<Assets, ApplicationError>;
    async fn get_asset(&self, asset_id: String) -> Result<Metadata, ApplicationError>;
    async fn get_asset_balance(&self, asset_id: String) -> Result<Balance, ApplicationError>;
    async fn send(
        &self,
        asset_id: String,
        recipients: Vec<Recipient>,
        donation: bool,
        fee_rate: f32,
        min_confirmations: u8,
    ) -> Result<String, ApplicationError>;
    async fn invoice(
        &self,
        asset_id: Option<String>,
        amount: Option<u64>,
        duration_seconds: Option<u32>,
        transport_endpoints: Vec<String>,
        min_confirmations: u8,
    ) -> Result<ReceiveData, ApplicationError>;
}

pub type DynRGBClient = Arc<dyn RGBClient + Send + Sync>;
