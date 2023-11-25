use std::sync::Arc;

use async_trait::async_trait;

use crate::{application::errors::ApplicationError, domains::rgb::entities::RGBContract};

#[async_trait]
pub trait RGBClient {
    async fn get_address(&self) -> Result<String, ApplicationError>;
    async fn get_btc_balance(&self) -> Result<u64, ApplicationError>;
    async fn send_btc(
        &self,
        address: String,
        amount: u64,
        fee_rate: f32,
    ) -> Result<String, ApplicationError>;
    async fn drain_btc(&self, address: String, fee_rate: f32) -> Result<String, ApplicationError>;
    async fn create_utxos(&self, fee_rate: f32) -> Result<u8, ApplicationError>;
    async fn issue_contract(&self, contract: RGBContract) -> Result<String, ApplicationError>;
}

pub type DynRGBClient = Arc<dyn RGBClient + Send + Sync>;
