use async_trait::async_trait;
use rgb_lib::wallet::{Assets, Balance, Metadata, ReceiveData, SendResult, Transfer, Unspent};

use crate::{application::errors::RGBError, domains::rgb::entities::RGBAsset};

#[async_trait]
pub trait RGBClient: Send + Sync {
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
    async fn issue_asset_nia(&self, contract: RGBAsset) -> Result<String, RGBError>;
    async fn issue_asset_cfa(&self, contract: RGBAsset) -> Result<String, RGBError>;
    async fn issue_asset_uda(&self, contract: RGBAsset) -> Result<String, RGBError>;
    async fn list_assets(&self) -> Result<Assets, RGBError>;
    async fn list_transfers(&self, asset_id: Option<String>) -> Result<Vec<Transfer>, RGBError>;
    async fn get_asset(&self, asset_id: String) -> Result<Metadata, RGBError>;
    async fn get_asset_balance(&self, asset_id: String) -> Result<Balance, RGBError>;
    async fn send(
        &self,
        asset_id: String,
        recipient_id: String,
        donation: bool,
        fee_rate: f32,
        amount: u64,
    ) -> Result<SendResult, RGBError>;
    async fn blind_receive(
        &self,
        asset_id: Option<String>,
        amount: Option<u64>,
        duration_seconds: Option<u32>,
    ) -> Result<ReceiveData, RGBError>;
    async fn witness_receive(
        &self,
        asset_id: Option<String>,
        amount: Option<u64>,
        duration_seconds: Option<u32>,
    ) -> Result<ReceiveData, RGBError>;
    async fn refresh(&self, asset_id: Option<String>) -> Result<(), RGBError>;
}
