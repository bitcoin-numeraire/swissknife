use std::{path::PathBuf, process, sync::Arc, time::Duration};

use breez_sdk_core::ReverseSwapInfo;
use reqwest::{
    header::{HeaderMap, HeaderValue},
    Certificate, Client, Response,
};
use serde::{de::DeserializeOwned, Deserialize, Serialize};
use serde_bolt::bitcoin::hashes::{sha256, Hash};
use tokio::fs;
use uuid::Uuid;

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
    pub maxfeepercent: Option<f64>,
    #[serde(deserialize_with = "deserialize_duration")]
    pub payment_timeout: Duration,
    pub payment_exemptfee: Option<u64>,
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
    maxfeepercent: Option<f64>,
    retry_for: Option<u32>,
    payment_exemptfee: Option<u64>,
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
            maxfeepercent: config.maxfeepercent,
            retry_for: Some(config.payment_timeout.as_secs() as u32),
            payment_exemptfee: config.payment_exemptfee,
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

    async fn post_request<T: DeserializeOwned>(
        &self,
        endpoint: &str,
        payload: &impl Serialize,
    ) -> anyhow::Result<T> {
        let response = self
            .client
            .post(format!("https://{}/v1/{}", self.base_url, endpoint))
            .json(payload)
            .send()
            .await?;

        Self::handle_response(response).await
    }

    async fn get_request<T: DeserializeOwned>(&self, endpoint: &str) -> anyhow::Result<T> {
        let response = self
            .client
            .get(format!("https://{}/v1/{}", self.base_url, endpoint))
            .send()
            .await?;

        Self::handle_response(response).await
    }

    async fn handle_response<T: DeserializeOwned>(response: Response) -> anyhow::Result<T> {
        let status = response.status();
        if status.is_client_error() || status.is_server_error() {
            let error_text = response.text().await?;
            if let Ok(error_response) = serde_json::from_str::<ErrorResponse>(&error_text) {
                return Err(anyhow::anyhow!(error_response.message));
            } else {
                return Err(anyhow::anyhow!(error_text));
            }
        }

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
        let mut request = InvoiceRequest {
            memo: description.clone(),
            expiry: expiry as u64,
            value_msat: amount_msat,
            ..Default::default()
        };

        if deschashonly {
            request.description_hash = sha256::Hash::hash(&description.as_bytes()).to_string();
        }

        let response: AddInvoiceResponse = self
            .post_request("invoices", &request)
            .await
            .map_err(|e| LightningError::Invoice(e.to_string()))?;

        let mut invoice: Invoice = response.into();
        invoice.id = Uuid::new_v4();

        Ok(invoice)
    }

    async fn pay(
        &self,
        bolt11: String,
        amount_msat: Option<u64>,
    ) -> Result<Payment, LightningError> {
        let response: PayResponse = self
            .post_request(
                "pay",
                &PayRequest {
                    bolt11,
                    amount_msat,
                    label: Some(Uuid::new_v4().to_string()),
                    maxfeepercent: self.maxfeepercent,
                    retry_for: self.retry_for,
                    exemptfee: self.payment_exemptfee,
                },
            )
            .await
            .map_err(|e| LightningError::Pay(e.to_string()))?;

        Ok(response.into())
    }

    async fn invoice_by_hash(
        &self,
        payment_hash: String,
    ) -> Result<Option<Invoice>, LightningError> {
        let result = self
            .get_request::<InvoiceResponse>(&format!("invoice/{}", payment_hash))
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
        self.get_request::<GetinfoResponse>("getinfo")
            .await
            .map_err(|e| LightningError::HealthCheck(e.to_string()))?;

        Ok(HealthStatus::Operational)
    }
}
