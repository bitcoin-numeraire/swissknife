use serde::Deserialize;
use std::{fs, io, path::PathBuf, sync::Arc};

use async_trait::async_trait;
use bip39::Mnemonic;
use breez_sdk_core::{
    BreezServices, ConnectRequest, EnvironmentType, GreenlightCredentials, GreenlightNodeConfig, NodeConfig,
    ReceivePaymentRequest, SendPaymentRequest,
};

use crate::{
    application::{
        entities::Ledger,
        errors::{BitcoinError, LightningError},
    },
    domains::{
        bitcoin::{
            BitcoinWallet, BtcAddressType, BtcNetwork, BtcOutput, BtcPreparedTransaction, BtcTransaction,
            OnchainSyncBatch, OnchainSyncCursor,
        },
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
        _label: String,
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

    async fn cancel_invoice(&self, _payment_hash: String, _label: String) -> Result<(), LightningError> {
        Err(LightningError::CancelInvoice(
            "Invoice cancellation is not supported for Breez".to_string(),
        ))
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

    async fn synchronize(&self, _cursor: Option<OnchainSyncCursor>) -> Result<OnchainSyncBatch, BitcoinError> {
        Ok(OnchainSyncBatch::default())
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
