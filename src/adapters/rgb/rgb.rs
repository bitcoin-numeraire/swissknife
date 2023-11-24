use std::sync::Arc;

use async_trait::async_trait;

use crate::{application::errors::ApplicationError, domains::rgb::entities::RGBContract};

#[async_trait]
pub trait RGBClient {
    async fn issue_contract(
        &self,
        url: String,
        contract: RGBContract,
    ) -> Result<String, ApplicationError>;
}

pub type DynRGBClient = Arc<dyn RGBClient + Send + Sync>;
