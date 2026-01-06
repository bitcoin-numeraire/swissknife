use std::{path::PathBuf, process, sync::Arc, time::Duration};

use anyhow::anyhow;
use breez_sdk_core::ReverseSwapInfo;
use chrono::Utc;
use futures_util::StreamExt;
use reqwest::{
    header::{HeaderMap, HeaderValue},
    Certificate, Client, Response,
};
use serde::{de::DeserializeOwned, Deserialize, Serialize};
use serde_bolt::bitcoin::hashes::{sha256, Hash};
use tokio::fs;
use uuid::Uuid;

use crate::{
    application::{
        entities::BitcoinWallet,
        errors::{BitcoinError, LightningError},
    },
    domains::{
        bitcoin::{BitcoinAddressType, BitcoinBalance, BitcoinNetwork, BitcoinOutput, BitcoinOutputStatus},
        invoice::Invoice,
        ln_node::LnEventsUseCases,
        payment::Payment,
        system::HealthStatus,
    },
    infra::{
        config::config_rs::deserialize_duration,
        lightning::{types::parse_network, LnClient},
    },
};
use async_trait::async_trait;
use base64::{engine::general_purpose::STANDARD, Engine};

use super::{lnd_types::*, lnd_websocket_client::listen_invoices};

#[derive(Clone, Debug, Deserialize)]
pub struct LndRestClientConfig {
    pub host: String,
    #[serde(deserialize_with = "deserialize_duration")]
    pub connect_timeout: Duration,
    pub connection_verbose: bool,
    #[serde(deserialize_with = "deserialize_duration")]
    pub timeout: Duration,
    pub accept_invalid_certs: bool,
    pub accept_invalid_hostnames: bool,
    pub fee_limit_msat: u64,
    #[serde(deserialize_with = "deserialize_duration")]
    pub payment_timeout: Duration,
    #[serde(deserialize_with = "deserialize_duration")]
    pub ws_min_reconnect_delay: Duration,
    #[serde(deserialize_with = "deserialize_duration")]
    pub ws_max_reconnect_delay: Duration,
    pub ca_cert_path: Option<String>,
    pub macaroon_path: String,
}

pub struct LndRestClient {
    client: Client,
    base_url: String,
    fee_limit_msat: u64,
    retry_for: u32,
}

const USER_AGENT: &str = "Numeraire Swissknife/1.0";

impl LndRestClient {
    pub async fn new(
        config: LndRestClientConfig,
        ln_events: Arc<dyn LnEventsUseCases>,
    ) -> Result<Self, LightningError> {
        let macaroon = Self::read_macaroon(&config.macaroon_path)
            .await
            .map_err(|e| LightningError::ParseConfig(e.to_string()))?;

        let mut headers = HeaderMap::new();
        let mut macaroon_header =
            HeaderValue::from_str(&macaroon).map_err(|e| LightningError::ParseConfig(e.to_string()))?;
        macaroon_header.set_sensitive(true);
        headers.insert("Grpc-Metadata-Macaroon", macaroon_header);

        let mut client_builder = Client::builder()
            .user_agent(USER_AGENT)
            .connect_timeout(config.connect_timeout)
            .timeout(config.timeout)
            .connection_verbose(config.connection_verbose)
            .default_headers(headers)
            .danger_accept_invalid_certs(config.accept_invalid_certs);

        if let Some(ca_cert_path) = &config.ca_cert_path {
            let ca_certificate = Self::read_ca(ca_cert_path)
                .await
                .map_err(|e| LightningError::ReadCertificates(e.to_string()))?;
            client_builder = client_builder
                .add_root_certificate(ca_certificate)
                .danger_accept_invalid_hostnames(config.accept_invalid_hostnames);
        }

        let client = client_builder
            .build()
            .map_err(|e| LightningError::ParseConfig(e.to_string()))?;

        let config_clone = config.clone();
        tokio::spawn(async move {
            if let Err(err) = listen_invoices(config_clone, macaroon, ln_events).await {
                tracing::error!(%err);
                process::exit(1);
            }
        });

        Ok(Self {
            client,
            base_url: config.host,
            fee_limit_msat: config.fee_limit_msat,
            retry_for: config.payment_timeout.as_secs() as u32,
        })
    }

    async fn read_ca(path: &str) -> anyhow::Result<Certificate> {
        let ca_file = fs::read(PathBuf::from(path)).await?;
        let ca_certificate = Certificate::from_pem(&ca_file)?;

        Ok(ca_certificate)
    }

    async fn read_macaroon(path: &str) -> anyhow::Result<String> {
        let macaroon_file = fs::read(PathBuf::from(path)).await?;

        // Check if the file is base64-encoded text or raw binary
        // Base64 files start with ASCII characters, raw binary macaroons start with 0x02
        let macaroon_bytes = if macaroon_file.first() == Some(&0x02) {
            // Raw binary format - use as-is
            macaroon_file
        } else {
            // Base64-encoded text - decode it first
            let base64_str = String::from_utf8(macaroon_file)?;
            STANDARD.decode(base64_str.trim())?
        };

        Ok(hex::encode(macaroon_bytes))
    }

    async fn post_request_buffered<T>(&self, endpoint: &str, payload: &impl Serialize) -> anyhow::Result<T>
    where
        T: DeserializeOwned,
    {
        let url = format!("https://{}/{}", self.base_url, endpoint);
        let mut request = self.client.post(url);
        request = request.json(payload);

        let response = request.send().await?;
        let response = Self::check_response_status(response).await?;

        // Buffer the stream
        let mut buffer = Vec::new();
        let mut stream = response.bytes_stream();

        while let Some(chunk) = stream.next().await {
            buffer.extend_from_slice(&chunk?);
        }

        // Deserialize the full response from the buffered data
        let result = serde_json::from_slice::<T>(&buffer)?;
        Ok(result)
    }

    async fn check_response_status(response: Response) -> anyhow::Result<Response> {
        let status = response.status();
        if status.is_client_error() || status.is_server_error() {
            let error_text = response.text().await?;
            if let Ok(error_response) = serde_json::from_str::<ErrorResponse>(&error_text) {
                return Err(anyhow!(error_response.message));
            } else {
                return Err(anyhow!(error_text));
            }
        }

        Ok(response)
    }

    async fn post_request<T>(&self, endpoint: &str, payload: &impl Serialize) -> anyhow::Result<T>
    where
        T: DeserializeOwned,
    {
        let url = format!("https://{}/{}", self.base_url, endpoint);
        let mut request = self.client.post(url);
        request = request.json(payload);

        let response = request.send().await?;
        let response = Self::check_response_status(response).await?;

        let result = response.json::<T>().await?;
        Ok(result)
    }

    async fn get_request<T>(&self, endpoint: &str) -> anyhow::Result<T>
    where
        T: DeserializeOwned,
    {
        let url = format!("https://{}/{}", self.base_url, endpoint);
        let request = self.client.get(url);

        let response = request.send().await?;
        let response = Self::check_response_status(response).await?;

        let result = response.json::<T>().await?;
        Ok(result)
    }

    fn parse_amount(value: Option<String>) -> i64 {
        value.and_then(|v| v.parse::<i64>().ok()).unwrap_or_default()
    }

    fn map_address_type(address_type: BitcoinAddressType) -> String {
        match address_type {
            BitcoinAddressType::P2sh => "NESTED_PUBKEY_HASH".to_string(),
            BitcoinAddressType::P2tr => "TAPROOT_PUBKEY".to_string(),
            _ => "WITNESS_PUBKEY_HASH".to_string(),
        }
    }
}

#[async_trait]
impl LnClient for LndRestClient {
    async fn disconnect(&self) -> Result<(), LightningError> {
        // TODO: Implement shutdown signal
        Ok(())
    }

    async fn invoice(
        &self,
        amount_msat: u64,
        description: String,
        expiry: u32,
        deschashonly: bool,
    ) -> Result<Invoice, LightningError> {
        let mut payload = InvoiceRequest {
            memo: description.clone(),
            expiry: expiry as u64,
            value_msat: amount_msat,
            ..Default::default()
        };

        if deschashonly {
            payload.description_hash = sha256::Hash::hash(description.as_bytes()).to_string();
        }

        let response: AddInvoiceResponse = self
            .post_request("v1/invoices", &payload)
            .await
            .map_err(|e| LightningError::Invoice(e.to_string()))?;

        Ok(response.into())
    }

    async fn pay(&self, bolt11: String, amount_msat: Option<u64>) -> Result<Payment, LightningError> {
        let payload = PayRequest {
            payment_request: bolt11,
            amt_msat: amount_msat,
            fee_limit_msat: self.fee_limit_msat,
            timeout_seconds: self.retry_for,
            no_inflight_updates: true,
        };

        let response: StreamPayResponse = self
            .post_request_buffered("v2/router/send", &payload)
            .await
            .map_err(|e| LightningError::Pay(e.to_string()))?;

        if let Some(result) = response.result {
            match result.status.as_str() {
                "SUCCEEDED" => Ok(result.into()),
                "FAILED" => Err(LightningError::Pay(result.failure_reason.to_string())),
                _ => Err(LightningError::UnexpectedStreamPayload(format!(
                    "Unexpected status {}",
                    result.status
                ))),
            }
        } else if let Some(error) = response.error {
            Err(LightningError::Pay(error.message))
        } else {
            Err(LightningError::UnexpectedStreamPayload(
                "Missing result or error field.".to_string(),
            ))
        }
    }

    async fn invoice_by_hash(&self, payment_hash: String) -> Result<Option<Invoice>, LightningError> {
        let result = self
            .get_request::<InvoiceResponse>(&format!("v1/invoice/{}", payment_hash))
            .await;

        match result {
            Ok(response) => Ok(Some(response.into())),
            Err(err) => {
                let err_msg = err.to_string();
                if err_msg.contains("there are no existing invoices") || err_msg.contains("unable to locate invoice") {
                    Ok(None)
                } else {
                    Err(LightningError::InvoiceByHash(err.to_string()))
                }
            }
        }
    }

    async fn health(&self) -> Result<HealthStatus, LightningError> {
        self.get_request::<GetinfoResponse>("v1/getinfo")
            .await
            .map_err(|e| LightningError::HealthCheck(e.to_string()))?;

        Ok(HealthStatus::Operational)
    }

    async fn pay_onchain(
        &self,
        _amount_sat: u64,
        _recipient_address: String,
        _feerate: u32,
    ) -> Result<ReverseSwapInfo, LightningError> {
        Err(LightningError::Unsupported(
            "Bitcoin payments are not implemented for LND".to_string(),
        ))
    }
}

#[async_trait]
impl BitcoinWallet for LndRestClient {
    async fn new_address(&self, address_type: BitcoinAddressType) -> Result<String, BitcoinError> {
        let response: NewAddressResponse = self
            .post_request(
                "v1/newaddress",
                &NewAddressRequest {
                    address_type: Self::map_address_type(address_type),
                },
            )
            .await
            .map_err(|e| BitcoinError::Address(e.to_string()))?;

        Ok(response.address)
    }

    async fn balance(&self) -> Result<BitcoinBalance, BitcoinError> {
        let response: WalletBalanceResponse = self
            .get_request("v1/balance/blockchain")
            .await
            .map_err(|e| BitcoinError::Balance(e.to_string()))?;

        Ok(BitcoinBalance {
            confirmed_sat: Self::parse_amount(response.confirmed_balance) as u64,
            unconfirmed_sat: Self::parse_amount(response.unconfirmed_balance) as u64,
        })
    }

    async fn send(&self, address: String, amount_sat: u64, fee_rate: Option<u32>) -> Result<String, BitcoinError> {
        let response: SendCoinsResponse = self
            .post_request(
                "v1/transactions",
                &SendCoinsRequest {
                    addr: address,
                    amount: amount_sat as i64,
                    sat_per_vbyte: fee_rate.map(|f| f as u64),
                },
            )
            .await
            .map_err(|e| BitcoinError::Transaction(e.to_string()))?;

        Ok(response.txid)
    }

    async fn list_outputs(&self) -> Result<Vec<BitcoinOutput>, BitcoinError> {
        let network = self.network().await?;
        let response: ListUnspentResponse = self
            .get_request("v2/wallet/utxos")
            .await
            .map_err(|e| BitcoinError::Outputs(e.to_string()))?;

        let outputs = response
            .utxos
            .unwrap_or_default()
            .into_iter()
            .filter_map(|utxo| {
                let outpoint = utxo.outpoint?;
                let txid = outpoint.txid_str?;
                let output_index = outpoint.output_index?;
                let amount_sat = utxo.amount_sat?;

                let status = match utxo.confirmations {
                    Some(confirmations) => {
                        if confirmations > 0 {
                            BitcoinOutputStatus::Confirmed
                        } else {
                            BitcoinOutputStatus::Unconfirmed
                        }
                    }
                    None => BitcoinOutputStatus::Unconfirmed,
                };

                Some(BitcoinOutput {
                    id: Uuid::new_v4(),
                    outpoint: format!("{txid}:{output_index}"),
                    txid,
                    output_index: output_index as u32,
                    address: utxo.address,
                    amount_sat: Self::parse_amount(Some(amount_sat)),
                    status,
                    timestamp: None,
                    network: network,
                    created_at: Utc::now(),
                    updated_at: None,
                })
            })
            .collect();

        Ok(outputs)
    }

    async fn network(&self) -> Result<BitcoinNetwork, BitcoinError> {
        let response: GetinfoResponse = self
            .get_request("v1/getinfo")
            .await
            .map_err(|e| BitcoinError::Network(e.to_string()))?;

        if let Some(chain) = response
            .chains
            .and_then(|mut chains| chains.pop())
            .and_then(|chain| chain.network)
        {
            return Ok(parse_network(&chain));
        }

        Err(BitcoinError::Network(
            "No chain information returned by LND".to_string(),
        ))
    }
}
