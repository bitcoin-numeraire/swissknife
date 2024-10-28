use std::{path::PathBuf, process, sync::Arc, time::Duration};

use anyhow::anyhow;
use breez_sdk_core::ReverseSwapInfo;
use bytes::Bytes;
use futures_util::{Stream, StreamExt};
use reqwest::{
    header::{HeaderMap, HeaderValue},
    Certificate, Client, Response,
};
use serde::{de::DeserializeOwned, Deserialize, Serialize};
use serde_bolt::bitcoin::hashes::{sha256, Hash};
use tokio::fs;

use crate::{
    application::errors::LightningError,
    domains::{
        invoice::Invoice, ln_node::LnEventsUseCases, payment::Payment, system::HealthStatus,
    },
    infra::{config::config_rs::deserialize_duration, lightning::LnClient},
};
use async_trait::async_trait;

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
        let mut macaroon_header = HeaderValue::from_str(&macaroon)
            .map_err(|e| LightningError::ParseConfig(e.to_string()))?;
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
        let macaroon_header = hex::encode(macaroon_file);

        Ok(macaroon_header)
    }

    async fn post_request_stream(
        &self,
        endpoint: &str,
        payload: &impl Serialize,
    ) -> anyhow::Result<impl Stream<Item = Result<Bytes, reqwest::Error>>> {
        let url = format!("https://{}/{}", self.base_url, endpoint);
        let mut request = self.client.post(url);
        request = request.json(payload);

        let response = request.send().await?;
        let response = Self::check_response_status(response).await?;

        Ok(response.bytes_stream())
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

    async fn pay(
        &self,
        bolt11: String,
        amount_msat: Option<u64>,
    ) -> Result<Payment, LightningError> {
        let payload = PayRequest {
            payment_request: bolt11,
            amt_msat: amount_msat,
            fee_limit_msat: self.fee_limit_msat,
            timeout_seconds: self.retry_for,
            no_inflight_updates: true,
        };

        let mut stream = self
            .post_request_stream("v2/router/send", &payload)
            .await
            .map_err(|e| LightningError::Pay(e.to_string()))?;

        while let Some(item) = stream.next().await {
            match item {
                Ok(bytes) => {
                    let message = serde_json::from_slice::<StreamPayResponse>(&bytes)
                        .map_err(|e| LightningError::ParseResponse(e.to_string()))?;

                    if let Some(response) = message.result {
                        match response.status.as_str() {
                            "SUCCEEDED" => {
                                return Ok(response.into());
                            }
                            "FAILED" => {
                                return Err(LightningError::Pay(
                                    response.failure_reason.to_string(),
                                ));
                            }
                            _ => {
                                // Continue listening to the stream
                            }
                        }
                    } else if let Some(error) = message.error {
                        return Err(LightningError::Pay(error.message));
                    } else {
                        return Err(LightningError::ParseResponse(
                            "Missing result or error field.".to_string(),
                        ));
                    }
                }
                Err(e) => {
                    return Err(LightningError::ParseResponse(e.to_string()));
                }
            }
        }

        Err(LightningError::Pay(
            "Payment did not return a final state.".to_string(),
        ))
    }

    async fn invoice_by_hash(
        &self,
        payment_hash: String,
    ) -> Result<Option<Invoice>, LightningError> {
        let result = self
            .get_request::<InvoiceResponse>(&format!("v1/invoice/{}", payment_hash))
            .await;

        match result {
            Ok(response) => Ok(Some(response.into())),
            Err(e) => {
                if e.to_string().contains("Not Found") {
                    Ok(None)
                } else {
                    Err(LightningError::InvoiceByHash(e.to_string()))
                }
            }
        }
    }

    async fn pay_onchain(
        &self,
        _amount_sat: u64,
        _recipient_address: String,
        _feerate: u32,
    ) -> Result<ReverseSwapInfo, LightningError> {
        todo!();
    }

    async fn health(&self) -> Result<HealthStatus, LightningError> {
        self.get_request::<GetinfoResponse>("v1/getinfo")
            .await
            .map_err(|e| LightningError::HealthCheck(e.to_string()))?;

        Ok(HealthStatus::Operational)
    }
}
