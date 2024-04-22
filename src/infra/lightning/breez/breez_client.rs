use serde::Deserialize;
use std::sync::Arc;

use async_trait::async_trait;
use bip39::Mnemonic;
use breez_sdk_core::{
    BreezServices, EnvironmentType, GreenlightNodeConfig, ListPaymentsRequest, LnUrlPayRequest,
    LnUrlPayRequestData, LnUrlPayResult, LspInformation, NodeConfig, NodeState, Payment,
    ReceivePaymentRequest, SendPaymentRequest, SendSpontaneousPaymentRequest,
};

use crate::{
    application::errors::LightningError,
    domains::lightning::entities::{LightningInvoice, LightningPayment},
    infra::lightning::LightningClient,
};

use super::BreezListener;

#[derive(Clone, Debug, Deserialize)]
pub struct BreezClientConfig {
    pub api_key: String,
    pub invite_code: String,
    pub working_dir: String,
    pub seed: String,
    pub domain: String,
}

pub struct BreezClient {
    sdk: Arc<BreezServices>,
}

impl BreezClient {
    pub async fn new(
        config: BreezClientConfig,
        listener: Box<BreezListener>,
    ) -> Result<Self, LightningError> {
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
            Mnemonic::parse(config.seed).map_err(|e| LightningError::ParseSeed(e.to_string()))?;

        let sdk = BreezServices::connect(breez_config.clone(), seed.to_seed("").to_vec(), listener)
            .await
            .map_err(|e| LightningError::Connect(e.to_string()))?;

        Ok(Self { sdk })
    }
}

#[async_trait]
impl LightningClient for BreezClient {
    async fn invoice(
        &self,
        amount_msat: u64,
        description: String,
    ) -> Result<LightningInvoice, LightningError> {
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

        Ok(response.ln_invoice.into())
    }

    fn node_info(&self) -> Result<NodeState, LightningError> {
        let node_info = self
            .sdk
            .node_info()
            .map_err(|e| LightningError::NodeInfo(e.to_string()))?;

        Ok(node_info)
    }

    async fn lsp_info(&self) -> Result<LspInformation, LightningError> {
        let lsp_info = self
            .sdk
            .lsp_info()
            .await
            .map_err(|e| LightningError::LSPInfo(e.to_string()))?;

        Ok(lsp_info)
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

    async fn send_payment(
        &self,
        bolt11: String,
        amount_msat: Option<u64>,
    ) -> Result<LightningPayment, LightningError> {
        let response = self
            .sdk
            .send_payment(SendPaymentRequest {
                bolt11,
                amount_msat,
            })
            .await
            .map_err(|e| LightningError::SendBolt11Payment(e.to_string()))?;

        Ok(response.payment.into())
    }

    async fn send_spontaneous_payment(
        &self,
        node_id: String,
        amount_msat: u64,
    ) -> Result<LightningPayment, LightningError> {
        let response = self
            .sdk
            .send_spontaneous_payment(SendSpontaneousPaymentRequest {
                node_id,
                amount_msat,
                extra_tlvs: None, // TODO: Add support for extra TLVs
            })
            .await
            .map_err(|e| LightningError::SendNodeIdPayment(e.to_string()))?;

        Ok(response.payment.into())
    }

    async fn lnurl_pay(
        &self,
        data: LnUrlPayRequestData,
        amount_msat: u64,
        comment: Option<String>,
    ) -> Result<LightningPayment, LightningError> {
        let result = self
            .sdk
            .lnurl_pay(LnUrlPayRequest {
                data,
                amount_msat,
                comment,
            })
            .await
            .map_err(|e| LightningError::SendLNURLPayment(e.to_string()))?;

        match result {
            LnUrlPayResult::EndpointSuccess { data } => Ok(LightningPayment {
                payment_hash: data.payment_hash,
                amount_msat,
                ..Default::default()
            }),
            LnUrlPayResult::EndpointError { data } => {
                return Err(LightningError::SendLNURLPayment(data.reason));
            }
            LnUrlPayResult::PayError { data } => Ok(LightningPayment {
                payment_hash: data.payment_hash,
                error: Some(data.reason),
                amount_msat,
                ..Default::default()
            }),
        }
    }

    async fn payment_by_hash(
        &self,
        payment_hash: String,
    ) -> Result<Option<Payment>, LightningError> {
        let response = self
            .sdk
            .payment_by_hash(payment_hash)
            .await
            .map_err(|e| LightningError::PaymentByHash(e.to_string()))?;

        Ok(response)
    }
}
