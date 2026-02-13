use serde::Deserialize;
use std::sync::Arc;

use async_trait::async_trait;
use breez_sdk_liquid::{
    model::{
        ConnectRequest, GetPaymentRequest, LiquidNetwork, PayAmount, PaymentMethod, PrepareReceiveRequest,
        PrepareSendRequest, ReceiveAmount, ReceivePaymentRequest, SendPaymentRequest,
    },
    sdk::LiquidSdk,
};

use crate::{
    application::errors::{BitcoinError, LightningError},
    domains::{
        bitcoin::{
            BitcoinWallet, BtcAddressType, BtcNetwork, BtcOutput, BtcPreparedTransaction, BtcTransaction,
            OnchainSyncBatch, OnchainSyncCursor,
        },
        invoice::Invoice,
        payment::Payment,
        system::HealthStatus,
    },
    infra::lightning::{breez::BreezListener, LnClient},
};

#[derive(Clone, Debug, Deserialize)]
pub struct BreezClientConfig {
    pub api_key: String,
    pub working_dir: String,
    pub mnemonic: String,
    pub passphrase: Option<String>,
    pub network: String,
    pub sync_service_url: Option<String>,
}

pub struct BreezClient {
    sdk: Arc<LiquidSdk>,
    network: BtcNetwork,
}

impl BreezClient {
    pub async fn new(config: BreezClientConfig, listener: BreezListener) -> Result<Self, LightningError> {
        let network = match config.network.to_lowercase().as_str() {
            "bitcoin" => (LiquidNetwork::Mainnet, BtcNetwork::Bitcoin),
            "testnet" => (LiquidNetwork::Testnet, BtcNetwork::Testnet),
            "regtest" => (LiquidNetwork::Regtest, BtcNetwork::Regtest),
            _ => return Err(LightningError::ParseConfig("Invalid network".to_string())),
        };
        let (liquid_network, btc_network) = network;

        let mut sdk_config = LiquidSdk::default_config(liquid_network, Some(config.api_key.clone()))
            .map_err(|e| LightningError::ParseConfig(e.to_string()))?;

        sdk_config.working_dir = config.working_dir.clone();

        if let Some(sync_service_url) = config
            .sync_service_url
            .as_deref()
            .map(str::trim)
            .filter(|value| !value.is_empty())
        {
            sdk_config.sync_service_url = Some(sync_service_url.to_string());
        }

        let sdk = LiquidSdk::connect(ConnectRequest {
            config: sdk_config,
            mnemonic: Some(config.mnemonic.clone()),
            passphrase: config.passphrase,
            seed: None,
        })
        .await
        .map_err(|e| LightningError::Connect(e.to_string()))?;

        sdk.add_event_listener(Box::new(listener))
            .await
            .map_err(|e| LightningError::Listener(e.to_string()))?;

        Ok(Self {
            sdk,
            network: btc_network,
        })
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
        label: String,
        expiry: u32,
        deschashonly: bool,
    ) -> Result<Invoice, LightningError> {
        let payer_amount_sat = (amount_msat / 1000).max(1);
        let prepare = self
            .sdk
            .prepare_receive_payment(&PrepareReceiveRequest {
                payment_method: PaymentMethod::Bolt11Invoice,
                amount: Some(ReceiveAmount::Bitcoin { payer_amount_sat }),
            })
            .await
            .map_err(|e| LightningError::Invoice(e.to_string()))?;

        let response = self
            .sdk
            .receive_payment(&ReceivePaymentRequest {
                prepare_response: prepare,
                description: Some(description),
                use_description_hash: Some(deschashonly),
                payer_note: if label.is_empty() { None } else { Some(label) },
            })
            .await
            .map_err(|e| LightningError::Invoice(e.to_string()))?;

        let parsed =
            LiquidSdk::parse_invoice(&response.destination).map_err(|e| LightningError::Invoice(e.to_string()))?;
        let mut invoice: Invoice = parsed.into();
        if expiry > 0 {
            invoice.ln_invoice = invoice.ln_invoice.map(|mut ln| {
                ln.expiry = std::time::Duration::from_secs(expiry as u64);
                ln
            });
        }
        Ok(invoice)
    }

    async fn pay(&self, bolt11: String, amount_msat: Option<u64>, label: String) -> Result<Payment, LightningError> {
        let prepare = self
            .sdk
            .prepare_send_payment(&PrepareSendRequest {
                destination: bolt11,
                amount: amount_msat.map(|msat| PayAmount::Bitcoin {
                    receiver_amount_sat: (msat / 1000).max(1),
                }),
            })
            .await
            .map_err(|e| LightningError::Pay(e.to_string()))?;

        let response = self
            .sdk
            .send_payment(&SendPaymentRequest {
                prepare_response: prepare,
                use_asset_fees: None,
                payer_note: if label.is_empty() { None } else { Some(label) },
            })
            .await
            .map_err(|e| LightningError::Pay(e.to_string()))?;

        Ok(response.payment.into())
    }

    async fn invoice_by_hash(&self, payment_hash: String) -> Result<Option<Invoice>, LightningError> {
        let response = self
            .sdk
            .get_payment(&GetPaymentRequest::PaymentHash { payment_hash })
            .await
            .map_err(|e| LightningError::InvoiceByHash(e.to_string()))?;

        Ok(response.map(Into::into))
    }

    async fn payment_by_hash(&self, payment_hash: String) -> Result<Option<Payment>, LightningError> {
        let response = self
            .sdk
            .get_payment(&GetPaymentRequest::PaymentHash { payment_hash })
            .await
            .map_err(|e| LightningError::Pay(e.to_string()))?;

        Ok(response.map(Into::into))
    }

    async fn cancel_invoice(&self, _payment_hash: String, _label: String) -> Result<(), LightningError> {
        Err(LightningError::CancelInvoice(
            "Invoice cancellation is not supported for Breez".to_string(),
        ))
    }

    async fn health(&self) -> Result<HealthStatus, LightningError> {
        self.sdk
            .get_info()
            .await
            .map_err(|e| LightningError::HealthCheck(e.to_string()))?;
        Ok(HealthStatus::Operational)
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
        self.network
    }
}
