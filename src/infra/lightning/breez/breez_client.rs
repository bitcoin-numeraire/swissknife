use serde::Deserialize;
use std::{fs, io, path::PathBuf, sync::Arc};
use uuid::Uuid;

use async_trait::async_trait;
use bip39::Mnemonic;
use breez_sdk_core::{
    BreezServices, ConnectRequest, EnvironmentType, GreenlightCredentials, GreenlightNodeConfig,
    ListPaymentsRequest, LnUrlPayRequest, LnUrlPayRequestData, LnUrlPayResult, LspInformation,
    NodeConfig, NodeState, Payment, ReceivePaymentRequest, SendPaymentRequest,
    SendSpontaneousPaymentRequest, ServiceHealthCheckResponse,
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
    pub working_dir: String,
    pub certs_dir: String,
    pub seed: String,
    pub log_in_file: bool,
}

const DEFAULT_CLIENT_CERT_FILENAME: &str = "client.crt";
const DEFAULT_CLIENT_KEY_FILENAME: &str = "client-key.pem";

pub struct BreezClient {
    api_key: String,
    sdk: Arc<BreezServices>,
}

impl BreezClient {
    pub async fn new(
        config: BreezClientConfig,
        listener: Box<BreezListener>,
    ) -> Result<Self, LightningError> {
        if config.log_in_file {
            BreezServices::init_logging(&config.working_dir, None)
                .map_err(|e| LightningError::Logging(e.to_string()))?;
        }

        let (client_key, client_crt) = Self::read_certificates(PathBuf::from(&config.certs_dir))
            .map_err(|e| LightningError::ReadCertificates(e.to_string()))?;

        let mut breez_config = BreezServices::default_config(
            EnvironmentType::Production,
            config.api_key.clone(),
            NodeConfig::Greenlight {
                config: GreenlightNodeConfig {
                    partner_credentials: Some(GreenlightCredentials {
                        device_cert: client_crt,
                        device_key: client_key,
                    }),
                    invite_code: None,
                },
            },
        );
        breez_config.working_dir = config.working_dir;

        let seed =
            Mnemonic::parse(config.seed).map_err(|e| LightningError::ParseSeed(e.to_string()))?;

        let sdk = BreezServices::connect(
            ConnectRequest {
                config: breez_config.clone(),
                seed: seed.to_seed("").to_vec(),
                restore_only: None,
            },
            listener,
        )
        .await
        .map_err(|e| LightningError::Connect(e.to_string()))?;

        Ok(Self {
            api_key: config.api_key.clone(),
            sdk,
        })
    }

    fn read_certificates(cert_dir: PathBuf) -> io::Result<(Vec<u8>, Vec<u8>)> {
        let key_path = cert_dir.join(DEFAULT_CLIENT_KEY_FILENAME);
        let crt_path = cert_dir.join(DEFAULT_CLIENT_CERT_FILENAME);

        let client_key = fs::read(key_path)?;
        let client_crt = fs::read(crt_path)?;

        Ok((client_key, client_crt))
    }
}

#[async_trait]
impl LightningClient for BreezClient {
    async fn invoice(
        &self,
        amount_msat: u64,
        description: String,
        expiry: u32,
    ) -> Result<LightningInvoice, LightningError> {
        let response = self
            .sdk
            .receive_payment(ReceivePaymentRequest {
                amount_msat,
                description,
                use_description_hash: Some(true),
                expiry: Some(expiry),
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
        label: Uuid,
    ) -> Result<LightningPayment, LightningError> {
        let response = self
            .sdk
            .send_payment(SendPaymentRequest {
                bolt11,
                amount_msat,
                label: Some(label.to_string()),
            })
            .await
            .map_err(|e| LightningError::SendBolt11Payment(e.to_string()))?;

        Ok(response.payment.into())
    }

    async fn send_spontaneous_payment(
        &self,
        node_id: String,
        amount_msat: u64,
        label: Uuid,
    ) -> Result<LightningPayment, LightningError> {
        let response = self
            .sdk
            .send_spontaneous_payment(SendSpontaneousPaymentRequest {
                node_id,
                amount_msat,
                extra_tlvs: None, // TODO: Add support for extra TLVs
                label: Some(label.to_string()),
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
        label: Uuid,
    ) -> Result<LightningPayment, LightningError> {
        let result = self
            .sdk
            .lnurl_pay(LnUrlPayRequest {
                data,
                amount_msat,
                comment,
                payment_label: Some(label.to_string()),
            })
            .await
            .map_err(|e| LightningError::SendLNURLPayment(e.to_string()))?;

        match result {
            LnUrlPayResult::EndpointSuccess { data } => {
                let mut payment: LightningPayment = data.payment.clone().into();
                payment.success_action = data
                    .success_action
                    .and_then(|action| serde_json::to_value(action).ok());

                Ok(payment)
            }
            LnUrlPayResult::EndpointError { data } => {
                return Err(LightningError::SendLNURLPayment(data.reason));
            }
            LnUrlPayResult::PayError { data } => Ok(LightningPayment {
                payment_hash: Some(data.payment_hash),
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

    async fn health(&self) -> Result<ServiceHealthCheckResponse, LightningError> {
        let response = BreezServices::service_health_check(self.api_key.clone())
            .await
            .map_err(|e| LightningError::HealthCheck(e.to_string()))?;

        Ok(response)
    }

    async fn list_lsps(&self) -> Result<Vec<LspInformation>, LightningError> {
        let response = self
            .sdk
            .list_lsps()
            .await
            .map_err(|e| LightningError::ListLSPs(e.to_string()))?;

        Ok(response)
    }
}
