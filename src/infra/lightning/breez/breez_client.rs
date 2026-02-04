use serde::Deserialize;
use serde_bolt::bitcoin::hashes::hex::ToHex;
use std::{fs, io, path::PathBuf, sync::Arc};
use tracing::info;

use async_trait::async_trait;
use bip39::Mnemonic;
use breez_sdk_core::{
    BreezServices, CheckMessageRequest, ConnectRequest, EnvironmentType, GreenlightCredentials, GreenlightNodeConfig,
    LspInformation, NodeConfig, NodeState, PrepareRedeemOnchainFundsRequest, ReceivePaymentRequest,
    RedeemOnchainFundsRequest, SendPaymentRequest, SignMessageRequest, StaticBackupRequest,
};

use crate::{
    application::{
        entities::Ledger,
        errors::{BitcoinError, LightningError},
    },
    domains::{
        bitcoin::{BitcoinWallet, BtcAddressType, BtcNetwork, BtcOutput, BtcPreparedTransaction, BtcTransaction},
        invoice::Invoice,
        payment::{Payment, PaymentStatus},
        system::HealthStatus,
    },
    infra::lightning::{breez::BreezListener, LnClient},
};

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
    working_dir: String,
    sdk: Arc<BreezServices>,
}

impl BreezClient {
    pub async fn new(config: BreezClientConfig, listener: BreezListener) -> Result<Self, LightningError> {
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
                        developer_cert: client_crt,
                        developer_key: client_key,
                    }),
                    invite_code: None,
                },
            },
        );
        breez_config.working_dir.clone_from(&config.working_dir);

        let seed = Mnemonic::parse(config.seed).map_err(|e| LightningError::ParseSeed(e.to_string()))?;

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
            api_key: config.api_key,
            working_dir: config.working_dir,
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
            .map_err(|e: breez_sdk_core::error::SdkError| LightningError::NodeInfo(e.to_string()))?;

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

    pub async fn redeem_onchain(&self, to_address: String, feerate: u32) -> Result<String, LightningError> {
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

    pub async fn check_message(
        &self,
        message: String,
        pubkey: String,
        signature: String,
    ) -> Result<bool, LightningError> {
        let response = self
            .sdk
            .check_message(CheckMessageRequest {
                message,
                pubkey,
                signature,
            })
            .await
            .map_err(|e| LightningError::CheckMessage(e.to_string()))?;

        Ok(response.is_valid)
    }

    pub async fn sync(&self) -> Result<(), LightningError> {
        self.sdk.sync().await.map_err(|e| LightningError::Sync(e.to_string()))
    }

    pub fn backup(&self) -> Result<Option<Vec<String>>, LightningError> {
        let status = self
            .sdk
            .backup_status()
            .map_err(|e| LightningError::Backup(e.to_string()))?;

        if !status.backed_up {
            return Ok(None);
        }

        let response = BreezServices::static_backup(StaticBackupRequest {
            working_dir: self.working_dir.clone(),
        })
        .map_err(|e| LightningError::Backup(e.to_string()))?;

        Ok(response.backup)
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
        deschashonly: bool,
    ) -> Result<Invoice, LightningError> {
        let response = self
            .sdk
            .receive_payment(ReceivePaymentRequest {
                amount_msat,
                description,
                use_description_hash: Some(deschashonly),
                expiry: Some(expiry),
                ..Default::default()
            })
            .await
            .map_err(|e| LightningError::Invoice(e.to_string()))?;

        Ok(response.ln_invoice.into())
    }

    async fn pay(&self, bolt11: String, amount_msat: Option<u64>, label: String) -> Result<Payment, LightningError> {
        let response = self
            .sdk
            .send_payment(SendPaymentRequest {
                bolt11,
                amount_msat,
                label: Some(label),
                use_trampoline: false,
            })
            .await
            .map_err(|e| LightningError::Pay(e.to_string()))?;

        Ok(response.payment.into())
    }

    async fn invoice_by_hash(&self, payment_hash: String) -> Result<Option<Invoice>, LightningError> {
        let response = self
            .sdk
            .payment_by_hash(payment_hash)
            .await
            .map_err(|e| LightningError::InvoiceByHash(e.to_string()))?;

        Ok(response.map(Into::into))
    }

    async fn payment_by_hash(&self, payment_hash: String) -> Result<Option<Payment>, LightningError> {
        let response = self
            .sdk
            .payment_by_hash(payment_hash)
            .await
            .map_err(|e| LightningError::Pay(e.to_string()))?;

        Ok(response.map(|payment| {
            let status = payment.status;
            let mut mapped: Payment = payment.into();
            mapped.ledger = Ledger::Lightning;
            mapped.status = match status {
                breez_sdk_core::PaymentStatus::Complete => PaymentStatus::Settled,
                breez_sdk_core::PaymentStatus::Failed => PaymentStatus::Failed,
                breez_sdk_core::PaymentStatus::Pending => PaymentStatus::Pending,
            };
            mapped
        }))
    }

    async fn health(&self) -> Result<HealthStatus, LightningError> {
        let response = BreezServices::service_health_check(self.api_key.clone())
            .await
            .map_err(|e| LightningError::HealthCheck(e.to_string()))?;

        Ok(response.status.into())
    }
}

#[async_trait]
impl BitcoinWallet for BreezClient {
    async fn new_address(&self, _address_type: BtcAddressType) -> Result<String, BitcoinError> {
        Err(BitcoinError::Unsupported(
            "Bitcoin address generation is not yet implemented for Breez".to_string(),
        ))
    }

    async fn prepare_transaction(
        &self,
        _address: String,
        _amount_sat: u64,
        _fee_rate: Option<u32>,
    ) -> Result<BtcPreparedTransaction, BitcoinError> {
        Err(BitcoinError::Unsupported(
            "Preparing bitcoin transactions is not yet implemented for Breez".to_string(),
        ))
    }

    async fn sign_send_transaction(&self, _prepared: &BtcPreparedTransaction) -> Result<(), BitcoinError> {
        Err(BitcoinError::Unsupported(
            "Broadcasting bitcoin transactions is not yet implemented for Breez".to_string(),
        ))
    }

    async fn release_prepared_transaction(&self, _prepared: &BtcPreparedTransaction) -> Result<(), BitcoinError> {
        Err(BitcoinError::Unsupported(
            "Releasing prepared bitcoin transactions is not yet implemented for Breez".to_string(),
        ))
    }

    async fn get_transaction(&self, _txid: &str) -> Result<Option<BtcTransaction>, BitcoinError> {
        Err(BitcoinError::Unsupported(
            "Transaction lookup is not yet implemented for Breez".to_string(),
        ))
    }

    async fn get_output(
        &self,
        _txid: &str,
        _output_index: Option<u32>,
        _address: Option<&str>,
        _include_spent: bool,
    ) -> Result<Option<BtcOutput>, BitcoinError> {
        Err(BitcoinError::Unsupported(
            "Output lookup is not yet implemented for Breez".to_string(),
        ))
    }

    fn network(&self) -> BtcNetwork {
        BtcNetwork::default()
    }
}
