use std::sync::Arc;

use async_trait::async_trait;
use breez_sdk_core::{
    BreezServices, Config, EnvironmentType, GreenlightNodeConfig, NodeConfig, ReceivePaymentRequest,
};

use crate::{
    adapters::lightning::LightningClient,
    application::errors::{ApplicationError, ConfigError, LightningError},
};

use super::BreezListener;

#[derive(Clone)]
pub struct BreezClientConfig {
    api_key: String,
    invite_code: Option<String>,
    working_dir: String,
    seed: Vec<u8>,
}

pub struct BreezClient {
    config: Config,
    sdk: Arc<BreezServices>,
}

impl BreezClient {
    pub async fn new(config: BreezClientConfig) -> Result<Self, ApplicationError> {
        let mut breez_config = BreezServices::default_config(
            EnvironmentType::Production,
            config.api_key,
            NodeConfig::Greenlight {
                config: GreenlightNodeConfig {
                    partner_credentials: None,
                    invite_code: config.invite_code,
                },
            },
        );
        breez_config.working_dir = config.working_dir;

        let sdk = BreezServices::connect(breez_config, config.seed, Box::new(BreezListener {}))
            .await
            .map_err(|e| ConfigError::Lightning(e.to_string()))?;

        Ok(Self {
            config: breez_config,
            sdk,
        })
    }
}

#[async_trait]
impl LightningClient for BreezClient {
    async fn invoice(
        &self,
        amount_msat: u64,
        description: String,
    ) -> Result<String, LightningError> {
        let response = self
            .sdk
            .receive_payment(ReceivePaymentRequest {
                amount_msat,
                description,
                use_description_hash: Some(true),
                ..Default::default()
            })
            .await
            .map_err(|e| LightningError::Invoice(e.to_string()))?;

        Ok(response.ln_invoice.bolt11)
    }
}
