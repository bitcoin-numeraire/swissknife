use serde::Deserialize;
use serde_bolt::bitcoin::hashes::hex::ToHex;
use std::{fs, io, path::PathBuf, sync::Arc};
use tracing::{debug, info};
use uuid::Uuid;

use async_trait::async_trait;
use bip39::Mnemonic;
use breez_sdk_core::{
    BreezServices, ConnectRequest, EnvironmentType, GreenlightCredentials, GreenlightNodeConfig,
    LspInformation, NodeConfig, NodeState, PayOnchainRequest, PrepareOnchainPaymentRequest,
    PrepareRedeemOnchainFundsRequest, ReceivePaymentRequest, RedeemOnchainFundsRequest,
    ReverseSwapInfo, SendPaymentRequest, ServiceHealthCheckResponse, SignMessageRequest,
    SwapAmountType,
};

use crate::{
    application::errors::LightningError,
    domains::{
        invoices::entities::Invoice, lightning::services::LnEventsUseCases,
        payments::entities::Payment,
    },
    infra::lightning::LnClient,
};

use super::BreezListener;

#[derive(Clone, Debug, Deserialize)]
pub struct BreezClientConfig {
    pub api_key: String,
    pub working_dir: String,
    pub certs_dir: String,
    pub seed: String,
    pub log_in_file: bool,
    pub restore_only: bool,
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
        ln_events: Arc<dyn LnEventsUseCases>,
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

        let listener = BreezListener::new(ln_events);

        let sdk = BreezServices::connect(
            ConnectRequest {
                config: breez_config.clone(),
                seed: seed.to_seed("").to_vec(),
                restore_only: Some(config.restore_only),
            },
            Box::new(listener),
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

    pub fn node_info(&self) -> Result<NodeState, LightningError> {
        let node_info = self
            .sdk
            .node_info()
            .map_err(|e: breez_sdk_core::error::SdkError| {
                LightningError::NodeInfo(e.to_string())
            })?;

        Ok(node_info)
    }

    pub async fn lsp_info(&self) -> Result<LspInformation, LightningError> {
        let lsp_info = self
            .sdk
            .lsp_info()
            .await
            .map_err(|e| LightningError::LSPInfo(e.to_string()))?;

        Ok(lsp_info)
    }

    pub async fn list_lsps(&self) -> Result<Vec<LspInformation>, LightningError> {
        let response = self
            .sdk
            .list_lsps()
            .await
            .map_err(|e| LightningError::ListLSPs(e.to_string()))?;

        Ok(response)
    }

    pub async fn close_lsp_channels(&self) -> Result<Vec<String>, LightningError> {
        let tx_ids = self
            .sdk
            .close_lsp_channels()
            .await
            .map_err(|e| LightningError::CloseLSPChannels(e.to_string()))?;

        Ok(tx_ids)
    }

    pub async fn redeem_onchain(
        &self,
        to_address: String,
        feerate: u32,
    ) -> Result<String, LightningError> {
        let prepare_res = self
            .sdk
            .prepare_redeem_onchain_funds(PrepareRedeemOnchainFundsRequest {
                to_address: to_address.clone(),
                sat_per_vbyte: feerate,
            })
            .await
            .map_err(|e| LightningError::RedeemOnChain(e.to_string()))?;

        info!(
            "Fees: {} sats, Weight: {} sats",
            prepare_res.tx_fee_sat, prepare_res.tx_weight,
        );

        let response = self
            .sdk
            .redeem_onchain_funds(RedeemOnchainFundsRequest {
                to_address,
                sat_per_vbyte: feerate,
            })
            .await
            .map_err(|e| LightningError::RedeemOnChain(e.to_string()))?;

        Ok(response.txid.to_hex())
    }

    pub async fn connect_lsp(&self, lsp_id: String) -> Result<(), LightningError> {
        self.sdk
            .connect_lsp(lsp_id)
            .await
            .map_err(|e| LightningError::ConnectLSP(e.to_string()))
    }

    pub async fn sign_message(&self, message: String) -> Result<String, LightningError> {
        let response = self
            .sdk
            .sign_message(SignMessageRequest { message })
            .await
            .map_err(|e| LightningError::SignMessage(e.to_string()))?;

        Ok(response.signature)
    }

    pub async fn sync(&self) -> Result<(), LightningError> {
        self.sdk
            .sync()
            .await
            .map_err(|e| LightningError::Sync(e.to_string()))
    }

    pub async fn backup(&self) -> Result<(), LightningError> {
        self.sdk
            .backup()
            .await
            .map_err(|e| LightningError::Backup(e.to_string()))
    }
}

#[async_trait]
impl LnClient for BreezClient {
    async fn disconnect(&self) -> Result<(), LightningError> {
        self.sdk
            .disconnect()
            .await
            .map_err(|e| LightningError::Disconnect(e.to_string()))
    }

    async fn invoice(
        &self,
        amount_msat: u64,
        description: String,
        expiry: u32,
    ) -> Result<Invoice, LightningError> {
        let response = self
            .sdk
            .receive_payment(ReceivePaymentRequest {
                amount_msat,
                description,
                use_description_hash: Some(false),
                expiry: Some(expiry),
                ..Default::default()
            })
            .await
            .map_err(|e| LightningError::Invoice(e.to_string()))?;

        Ok(response.ln_invoice.into())
    }

    async fn pay(
        &self,
        bolt11: String,
        amount_msat: Option<u64>,
    ) -> Result<Payment, LightningError> {
        let response = self
            .sdk
            .send_payment(SendPaymentRequest {
                bolt11,
                amount_msat,
                label: Some(Uuid::new_v4().to_string()),
            })
            .await
            .map_err(|e| LightningError::Pay(e.to_string()))?;

        Ok(response.payment.into())
    }

    async fn pay_onchain(
        &self,
        amount_sat: u64,
        recipient_address: String,
        feerate: u32,
    ) -> Result<ReverseSwapInfo, LightningError> {
        let current_limits = self
            .sdk
            .onchain_payment_limits()
            .await
            .map_err(|e| LightningError::PayOnChain(e.to_string()))?;

        debug!(
            "Minimum amount: {} sats, Maximum amount: {} sats",
            current_limits.min_sat, current_limits.max_sat
        );

        let prepare_res = self
            .sdk
            .prepare_onchain_payment(PrepareOnchainPaymentRequest {
                amount_sat,
                amount_type: SwapAmountType::Send,
                claim_tx_feerate: feerate,
            })
            .await
            .map_err(|e| LightningError::PayOnChain(e.to_string()))?;

        info!(
            "Sender amount: {} sats, Recipient amount: {} sats, Total fees: {} sats",
            prepare_res.sender_amount_sat, prepare_res.recipient_amount_sat, prepare_res.total_fees
        );

        let response = self
            .sdk
            .pay_onchain(PayOnchainRequest {
                recipient_address,
                prepare_res,
            })
            .await
            .map_err(|e| LightningError::PayOnChain(e.to_string()))?;

        Ok(response.reverse_swap_info)
    }

    async fn health(&self) -> Result<ServiceHealthCheckResponse, LightningError> {
        let response = BreezServices::service_health_check(self.api_key.clone())
            .await
            .map_err(|e| LightningError::HealthCheck(e.to_string()))?;

        Ok(response)
    }
}
