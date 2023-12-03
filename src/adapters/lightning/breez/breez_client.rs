use serde::Deserialize;
use std::sync::Arc;

use async_trait::async_trait;
use bip39::Mnemonic;
use breez_sdk_core::{
    BreezServices, EnvironmentType, GreenlightNodeConfig, ListPaymentsRequest, NodeConfig,
    NodeState, Payment, ReceivePaymentRequest,
};

use crate::{
    adapters::lightning::LightningClient,
    application::errors::{ApplicationError, ConfigError, LightningError},
};

use super::BreezListener;

#[derive(Clone, Debug, Deserialize)]
pub struct BreezClientConfig {
    pub api_key: String,
    pub invite_code: String,
    pub working_dir: String,
    pub seed: String,
}

pub struct BreezClient {
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
                    invite_code: Some(config.invite_code),
                },
            },
        );
        breez_config.working_dir = config.working_dir;

        let seed =
            Mnemonic::parse(config.seed).map_err(|e| ConfigError::Lightning(e.to_string()))?;

        let sdk = BreezServices::connect(
            breez_config,
            seed.to_seed("").to_vec(),
            Box::new(BreezListener {}),
        )
        .await
        .map_err(|e| ConfigError::Lightning(e.to_string()))?;

        Ok(Self { sdk })
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

    async fn node_info(&self) -> Result<NodeState, LightningError> {
        let node_info = self
            .sdk
            .node_info()
            .map_err(|e| LightningError::NodeInfo(e.to_string()))?;

        Ok(node_info)
    }

    async fn list_payments(&self) -> Result<Vec<Payment>, LightningError> {
        let payments = self
            .sdk
            .list_payments(ListPaymentsRequest {
                ..Default::default()
            })
            .await
            .map_err(|e| LightningError::ListPayments(e.to_string()))?;

        Ok(payments)
    }
}