use std::{path::PathBuf, time::Duration};

use anyhow::anyhow;
use breez_sdk_core::ReverseSwapInfo;
use chrono::TimeZone;
use futures_util::StreamExt;
use reqwest::{
    header::{HeaderMap, HeaderValue},
    Certificate, Client, Response,
};
use serde::{de::DeserializeOwned, Deserialize, Serialize};
use serde_bolt::bitcoin::hashes::{sha256, Hash};
use tokio::fs;

use crate::{
    application::{
        entities::Ledger,
        errors::{BitcoinError, LightningError},
    },
    domains::{
        bitcoin::{BitcoinWallet, BtcAddressType, BtcNetwork, BtcOutput, BtcOutputStatus, BtcTransaction},
        invoice::Invoice,
        payment::{LnPayment, Payment, PaymentStatus},
        system::HealthStatus,
    },
    infra::{
        config::config_rs::deserialize_duration,
        lightning::{types::parse_network, LnClient},
    },
};
use async_trait::async_trait;
use base64::{engine::general_purpose::STANDARD, Engine};

use super::lnd_types::*;

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
    network: BtcNetwork,
}

const USER_AGENT: &str = "Numeraire Swissknife/1.0";

impl LndRestClient {
    pub async fn new(config: LndRestClientConfig) -> Result<Self, LightningError> {
        let macaroon = read_macaroon(&config.macaroon_path)
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

        let mut lnd_client = Self {
            client,
            base_url: config.host.clone(),
            fee_limit_msat: config.fee_limit_msat,
            retry_for: config.payment_timeout.as_secs() as u32,
            network: BtcNetwork::default(),
        };

        let network = lnd_client.network().await?;
        lnd_client.network = network;

        Ok(lnd_client)
    }

    async fn read_ca(path: &str) -> anyhow::Result<Certificate> {
        let ca_file = fs::read(PathBuf::from(path)).await?;
        let ca_certificate = Certificate::from_pem(&ca_file)?;

        Ok(ca_certificate)
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

    pub async fn track_payment(&self, payment_hash: &str) -> Result<Option<TrackPaymentResponse>, LightningError> {
        let endpoint = format!("v2/router/track/{}", payment_hash);
        let payload = TrackPaymentRequest {
            no_inflight_updates: true,
        };

        let result = self
            .post_request_buffered::<TrackPaymentResponse>(&endpoint, &payload)
            .await;

        match result {
            Ok(response) => Ok(Some(response)),
            Err(err) => {
                let err_msg = err.to_string();
                if err_msg.contains("unable to find payment") || err_msg.contains("unknown payment") {
                    Ok(None)
                } else {
                    Err(LightningError::Sync(err.to_string()))
                }
            }
        }
    }

    async fn network(&self) -> Result<BtcNetwork, LightningError> {
        let response: GetinfoResponse = self
            .get_request("v1/getinfo")
            .await
            .map_err(|e| LightningError::NodeInfo(e.to_string()))?;

        if let Some(chain) = response
            .chains
            .and_then(|mut chains| chains.pop())
            .and_then(|chain| chain.network)
        {
            return Ok(parse_network(&chain));
        }

        Err(LightningError::NodeInfo(
            "No chain information returned by LND".to_string(),
        ))
    }
}

pub(crate) async fn read_macaroon(path: &str) -> anyhow::Result<String> {
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

    async fn pay(&self, bolt11: String, amount_msat: Option<u64>, _label: String) -> Result<Payment, LightningError> {
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

    async fn payment_by_hash(&self, payment_hash: String) -> Result<Option<Payment>, LightningError> {
        let status = self.track_payment(&payment_hash).await?;
        let Some(status) = status else {
            return Ok(None);
        };

        match status.status.as_str() {
            "SUCCEEDED" => {
                let payment_time_ns = status.payment_time_ns.or(status.creation_time_ns).unwrap_or_default();
                Ok(Some(Payment {
                    ledger: Ledger::Lightning,
                    status: PaymentStatus::Settled,
                    amount_msat: status.value_msat.unwrap_or_default(),
                    fee_msat: status.fee_msat,
                    payment_time: Some(chrono::Utc.timestamp_nanos(payment_time_ns)),
                    lightning: Some(LnPayment {
                        payment_hash: Some(status.payment_hash),
                        payment_preimage: Some(status.payment_preimage),
                        ..Default::default()
                    }),
                    ..Default::default()
                }))
            }
            "FAILED" => Ok(Some(Payment {
                ledger: Ledger::Lightning,
                status: PaymentStatus::Failed,
                error: Some(status.failure_reason),
                amount_msat: status.value_msat.unwrap_or_default(),
                fee_msat: status.fee_msat,
                lightning: Some(LnPayment {
                    payment_hash: Some(status.payment_hash),
                    ..Default::default()
                }),
                ..Default::default()
            })),
            _ => Ok(None),
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
    async fn new_address(&self, address_type: BtcAddressType) -> Result<String, BitcoinError> {
        let address_type_param = match address_type {
            BtcAddressType::P2sh => "NESTED_PUBKEY_HASH".to_string(),
            BtcAddressType::P2tr => "TAPROOT_PUBKEY".to_string(),
            BtcAddressType::P2wpkh => "WITNESS_PUBKEY_HASH".to_string(),
            _ => return Err(BitcoinError::AddressType(address_type.to_string())),
        };

        let endpoint = format!("v1/newaddress?type={}", address_type_param);

        let response: NewAddressResponse = self
            .get_request(&endpoint)
            .await
            .map_err(|e| BitcoinError::Address(e.to_string()))?;

        Ok(response.address)
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

    async fn get_transaction(&self, txid: &str) -> Result<BtcTransaction, BitcoinError> {
        let endpoint = format!("v2/wallet/transactions/{txid}");
        let response: TransactionResponse = self
            .get_request(&endpoint)
            .await
            .map_err(|e| BitcoinError::Transaction(e.to_string()))?;

        Ok(response.into())
    }

    async fn get_output(
        &self,
        txid: &str,
        output_index: Option<u32>,
        address: Option<&str>,
    ) -> Result<Option<BtcOutput>, BitcoinError> {
        let transaction = self.get_transaction(txid).await?;
        let output = transaction.outputs.iter().find(|output| match output_index {
            Some(index) => output.output_index == index,
            None => address
                .and_then(|target| output.address.as_deref().map(|addr| addr == target))
                .unwrap_or(false),
        });

        Ok(output.map(|output| BtcOutput {
            txid: transaction.txid.clone(),
            output_index: output.output_index,
            address: output.address.clone().unwrap_or_default(),
            amount_sat: output.amount_sat,
            block_height: transaction.block_height,
            network: self.network,
            outpoint: format!("{}:{}", transaction.txid, output.output_index),
            status: if transaction.block_height.is_some() && transaction.block_height.unwrap() > 0 {
                BtcOutputStatus::Confirmed
            } else {
                BtcOutputStatus::Unconfirmed
            },
            ..Default::default()
        }))
    }

    fn network(&self) -> BtcNetwork {
        self.network
    }
}
