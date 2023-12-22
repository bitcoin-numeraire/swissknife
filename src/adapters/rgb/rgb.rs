use std::sync::Arc;

use async_trait::async_trait;
use rgb_lib::wallet::{Assets, Balance, Metadata, ReceiveData, Recipient, Unspent};

use crate::{application::errors::RGBError, domains::rgb::entities::RGBContract};

#[async_trait]
pub trait RGBClient {
    async fn get_address(&self) -> Result<String, RGBError>;
    async fn get_btc_balance(&self) -> Result<u64, RGBError>;
    async fn list_unspents(&self) -> Result<Vec<Unspent>, RGBError>;
    async fn send_btc(
        &self,
        address: String,
        amount: u64,
        fee_rate: f32,
    ) -> Result<String, RGBError>;
    async fn drain_btc(&self, address: String, fee_rate: f32) -> Result<String, RGBError>;
    async fn create_utxos(&self, fee_rate: f32) -> Result<u8, RGBError>;
    async fn issue_contract(&self, contract: RGBContract) -> Result<String, RGBError>;
    async fn list_assets(&self) -> Result<Assets, RGBError>;
    async fn get_asset(&self, asset_id: String) -> Result<Metadata, RGBError>;
    async fn get_asset_balance(&self, asset_id: String) -> Result<Balance, RGBError>;
    async fn send(
        &self,
        asset_id: String,
        recipients: Vec<Recipient>,
        donation: bool,
        fee_rate: f32,
        min_confirmations: u8,
    ) -> Result<String, RGBError>;
    async fn invoice(
        &self,
        asset_id: Option<String>,
        amount: Option<u64>,
        duration_seconds: Option<u32>,
        transport_endpoints: Vec<String>,
        min_confirmations: u8,
    ) -> Result<ReceiveData, RGBError>;
}

pub type DynRGBClient = Arc<dyn RGBClient + Send + Sync>;
