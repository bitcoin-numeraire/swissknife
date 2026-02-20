use serde::Deserialize;
use std::collections::HashMap;
use std::str::FromStr;
use std::sync::Arc;

use async_trait::async_trait;
use breez_sdk_spark::{
    connect, default_config, BreezSdk, ConnectRequest, GetInfoRequest, ListPaymentsRequest, Network as SparkNetwork,
    PaymentDetails as SparkPaymentDetails, PrepareSendPaymentRequest, PrepareSendPaymentResponse,
    ReceivePaymentMethod, ReceivePaymentRequest, Seed, SendPaymentRequest,
};
use chrono::TimeZone;
use lightning_invoice::Bolt11Invoice;
use tokio::sync::Mutex;

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
        event::EventService,
        invoice::{Invoice, LnInvoice},
        payment::Payment,
        system::HealthStatus,
    },
    infra::lightning::LnClient,
};

use super::breez_spark_listener::BreezListener;

#[derive(Clone, Debug, Deserialize)]
pub struct BreezClientConfig {
    pub api_key: Option<String>,
    pub working_dir: String,
    pub mnemonic: String,
    pub passphrase: Option<String>,
    pub network: String,
    pub sync_interval_secs: Option<u32>,
    pub prefer_spark_over_lightning: Option<bool>,
    pub real_time_sync_server_url: Option<String>,
    pub lnurl_domain: Option<String>,
}

pub struct BreezClient {
    sdk: BreezSdk,
    network: BtcNetwork,
    /// Stores prepared on-chain withdrawal responses keyed by UUID.
    /// Used to bridge the two-step prepare/sign flow of the BitcoinWallet trait
    /// without requiring serialization of SDK-internal types.
    pending_sends: Arc<Mutex<HashMap<String, PrepareSendPaymentResponse>>>,
}

impl BreezClient {
    pub async fn new(config: BreezClientConfig, events: EventService) -> Result<Self, LightningError> {
        let (spark_network, btc_network) = match config.network.to_lowercase().as_str() {
            "bitcoin" | "mainnet" => (SparkNetwork::Mainnet, BtcNetwork::Bitcoin),
            "regtest" => (SparkNetwork::Regtest, BtcNetwork::Regtest),
            _ => {
                return Err(LightningError::ParseConfig(
                    "Invalid network. Spark supports 'bitcoin' (mainnet) and 'regtest' only.".to_string(),
                ))
            }
        };

        let listener = BreezListener::new(events, btc_network.into());

        let mut sdk_config = default_config(spark_network);
        sdk_config.api_key = config.api_key.clone();

        if let Some(secs) = config.sync_interval_secs {
            sdk_config.sync_interval_secs = secs;
        }
        if let Some(prefer) = config.prefer_spark_over_lightning {
            sdk_config.prefer_spark_over_lightning = prefer;
        }
        if let Some(url) = config
            .real_time_sync_server_url
            .as_deref()
            .map(str::trim)
            .filter(|v| !v.is_empty())
        {
            sdk_config.real_time_sync_server_url = Some(url.to_string());
        }
        if let Some(domain) = config
            .lnurl_domain
            .as_deref()
            .map(str::trim)
            .filter(|v| !v.is_empty())
        {
            sdk_config.lnurl_domain = Some(domain.to_string());
        }

        let sdk = connect(ConnectRequest {
            config: sdk_config,
            seed: Seed::Mnemonic {
                mnemonic: config.mnemonic.clone(),
                passphrase: config.passphrase,
            },
            storage_dir: config.working_dir.clone(),
        })
        .await
        .map_err(|e| LightningError::Connect(e.to_string()))?;

        sdk.add_event_listener(Box::new(listener));

        Ok(Self {
            sdk,
            network: btc_network,
            pending_sends: Arc::new(Mutex::new(HashMap::new())),
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
        _label: String,
        expiry: u32,
        _deschashonly: bool,
    ) -> Result<Invoice, LightningError> {
        let amount_sats = (amount_msat / 1000).max(1);

        let response = self
            .sdk
            .receive_payment(ReceivePaymentRequest {
                payment_method: ReceivePaymentMethod::Bolt11Invoice {
                    description,
                    amount_sats: Some(amount_sats),
                    expiry_secs: if expiry > 0 { Some(expiry) } else { None },
                    payment_hash: None,
                },
            })
            .await
            .map_err(|e| LightningError::Invoice(e.to_string()))?;

        // Parse the bolt11 string returned by the SDK to extract invoice details
        let bolt11_str = response.payment_request;
        let parsed = Bolt11Invoice::from_str(&bolt11_str)
            .map_err(|e| LightningError::Invoice(format!("Failed to parse bolt11: {e}")))?;

        let created_at = chrono::Utc
            .timestamp_opt(parsed.duration_since_epoch().as_secs() as i64, 0)
            .single()
            .unwrap_or_else(chrono::Utc::now);
        let expiry_duration = parsed.expiry_time();
        let expires_at = created_at + chrono::Duration::from_std(expiry_duration).unwrap_or_default();

        let description_text = match parsed.description() {
            lightning_invoice::Bolt11InvoiceDescription::Direct(desc) => Some(desc.to_string()),
            lightning_invoice::Bolt11InvoiceDescription::Hash(_) => None,
        };
        let description_hash = match parsed.description() {
            lightning_invoice::Bolt11InvoiceDescription::Hash(hash) => Some(hash.0.to_string()),
            _ => None,
        };

        Ok(Invoice {
            ledger: Ledger::Lightning,
            description: description_text,
            amount_msat: parsed.amount_milli_satoshis(),
            timestamp: created_at,
            ln_invoice: Some(LnInvoice {
                bolt11: bolt11_str,
                payee_pubkey: parsed.recover_payee_pub_key().to_string(),
                payment_hash: parsed.payment_hash().to_string(),
                description_hash,
                payment_secret: hex::encode(parsed.payment_secret().0),
                min_final_cltv_expiry_delta: parsed.min_final_cltv_expiry_delta(),
                expiry: expiry_duration,
                expires_at,
            }),
            ..Default::default()
        })
    }

    async fn pay(&self, bolt11: String, amount_msat: Option<u64>, _label: String) -> Result<Payment, LightningError> {
        let prepare = self
            .sdk
            .prepare_send_payment(PrepareSendPaymentRequest {
                payment_request: bolt11,
                amount: amount_msat.map(|msat| ((msat / 1000).max(1)) as u128),
                token_identifier: None,
                conversion_options: None,
                fee_policy: None,
            })
            .await
            .map_err(|e| LightningError::Pay(e.to_string()))?;

        let response = self
            .sdk
            .send_payment(SendPaymentRequest {
                prepare_response: prepare,
                options: None,
                idempotency_key: None,
            })
            .await
            .map_err(|e| LightningError::Pay(e.to_string()))?;

        Ok(response.payment.into())
    }

    async fn invoice_by_hash(&self, payment_hash: String) -> Result<Option<Invoice>, LightningError> {
        let response = self
            .sdk
            .list_payments(ListPaymentsRequest::default())
            .await
            .map_err(|e| LightningError::InvoiceByHash(e.to_string()))?;

        let payment = response.payments.into_iter().find(|p| {
            matches!(
                &p.details,
                Some(SparkPaymentDetails::Lightning { htlc_details, .. })
                if htlc_details.payment_hash == payment_hash
            )
        });

        Ok(payment.map(Into::into))
    }

    async fn payment_by_hash(&self, payment_hash: String) -> Result<Option<Payment>, LightningError> {
        let response = self
            .sdk
            .list_payments(ListPaymentsRequest::default())
            .await
            .map_err(|e| LightningError::PaymentByHash(e.to_string()))?;

        let payment = response.payments.into_iter().find(|p| {
            matches!(
                &p.details,
                Some(SparkPaymentDetails::Lightning { htlc_details, .. })
                if htlc_details.payment_hash == payment_hash
            )
        });

        Ok(payment.map(Into::into))
    }

    async fn cancel_invoice(&self, _payment_hash: String, _label: String) -> Result<(), LightningError> {
        // Spark doesn't support explicit invoice cancellation
        Ok(())
    }

    async fn health(&self) -> Result<HealthStatus, LightningError> {
        self.sdk
            .get_info(GetInfoRequest::default())
            .await
            .map_err(|e| LightningError::HealthCheck(e.to_string()))?;

        Ok(HealthStatus::Operational)
    }
}

#[async_trait]
impl BitcoinWallet for BreezClient {
    async fn new_address(&self, _address_type: BtcAddressType) -> Result<String, BitcoinError> {
        // Spark generates its own deposit address type (likely P2TR/Taproot).
        // The address_type parameter is ignored since Spark controls address generation.
        let response = self
            .sdk
            .receive_payment(ReceivePaymentRequest {
                payment_method: ReceivePaymentMethod::BitcoinAddress,
            })
            .await
            .map_err(|e| BitcoinError::Address(e.to_string()))?;

        // The response may be a BIP21 URI; extract the plain address.
        let address = parse_bip21_address(&response.payment_request);
        Ok(address)
    }

    async fn prepare_transaction(
        &self,
        address: String,
        amount_sat: u64,
        _fee_rate: Option<u32>,
    ) -> Result<BtcPreparedTransaction, BitcoinError> {
        let prepare_response = self
            .sdk
            .prepare_send_payment(PrepareSendPaymentRequest {
                payment_request: address,
                amount: Some(amount_sat as u128),
                token_identifier: None,
                conversion_options: None,
                fee_policy: None,
            })
            .await
            .map_err(|e| BitcoinError::PrepareTransaction(e.to_string()))?;

        let key = uuid::Uuid::new_v4().to_string();

        // Estimate fee as the difference between what SDK charges and what the receiver gets.
        // This is a best-effort approximation; actual fee depends on the SDK's fee policy.
        let total_amount = prepare_response.amount as u64;
        let fee_sat = total_amount.saturating_sub(amount_sat);

        self.pending_sends
            .lock()
            .await
            .insert(key.clone(), prepare_response);

        Ok(BtcPreparedTransaction {
            txid: format!("breez-withdraw-{key}"),
            fee_sat,
            psbt: key,
            locked_utxos: Vec::new(),
        })
    }

    async fn sign_send_transaction(&self, prepared: &BtcPreparedTransaction) -> Result<Option<String>, BitcoinError> {
        let prepare_response = self
            .pending_sends
            .lock()
            .await
            .remove(&prepared.psbt)
            .ok_or_else(|| {
                BitcoinError::FinalizeTransaction(
                    "Prepared transaction not found or already consumed".to_string(),
                )
            })?;

        let response = self
            .sdk
            .send_payment(SendPaymentRequest {
                prepare_response,
                options: None,
                idempotency_key: None,
            })
            .await
            .map_err(|e| BitcoinError::BroadcastTransaction(e.to_string()))?;

        // Extract the Bitcoin tx_id from Withdraw payment details.
        let txid = match &response.payment.details {
            Some(SparkPaymentDetails::Withdraw { tx_id }) => Some(tx_id.clone()),
            _ => None,
        };

        Ok(txid)
    }

    async fn release_prepared_transaction(&self, prepared: &BtcPreparedTransaction) -> Result<(), BitcoinError> {
        self.pending_sends.lock().await.remove(&prepared.psbt);
        Ok(())
    }

    async fn get_transaction(&self, _txid: &str) -> Result<Option<BtcTransaction>, BitcoinError> {
        Err(BitcoinError::Unsupported(
            "Get transaction for Breez Spark".to_string(),
        ))
    }

    async fn synchronize(&self, cursor: Option<OnchainSyncCursor>) -> Result<OnchainSyncBatch, BitcoinError> {
        // Spark SDK auto-syncs based on sync_interval_secs.
        // Triggering get_info ensures latest state is available.
        self.sdk
            .get_info(GetInfoRequest::default())
            .await
            .map_err(|e| BitcoinError::Synchronize(e.to_string()))?;

        Ok(OnchainSyncBatch {
            events: vec![],
            next_cursor: cursor,
        })
    }

    async fn get_output(
        &self,
        _txid: &str,
        _output_index: Option<u32>,
        _address: Option<&str>,
        _include_spent: bool,
    ) -> Result<Option<BtcOutput>, BitcoinError> {
        Err(BitcoinError::Unsupported(
            "Get output for Breez Spark".to_string(),
        ))
    }

    fn network(&self) -> BtcNetwork {
        self.network
    }
}

/// Extracts the plain Bitcoin address from a BIP21 URI.
/// e.g. "bitcoin:bc1p...?amount=0.001&label=..." → "bc1p..."
/// If the input is not a BIP21 URI, returns it as-is.
fn parse_bip21_address(destination: &str) -> String {
    let stripped = destination
        .strip_prefix("bitcoin:")
        .or_else(|| destination.strip_prefix("BITCOIN:"))
        .unwrap_or(destination);

    stripped.split('?').next().unwrap_or(stripped).to_string()
}
