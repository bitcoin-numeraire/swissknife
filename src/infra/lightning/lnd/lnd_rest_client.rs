use std::{path::PathBuf, str::FromStr, sync::Arc, time::Duration};

use breez_sdk_core::ReverseSwapInfo;
use lightning_invoice::Bolt11Invoice;
use reqwest::{
    header::{HeaderMap, HeaderValue},
    Certificate, Client,
};
use rust_socketio::asynchronous::Client as WsClient;
use serde::{de::DeserializeOwned, Deserialize, Serialize};
use serde_bolt::bitcoin::hashes::{sha256, Hash};
use tokio::fs;
use uuid::Uuid;

use async_trait::async_trait;

use crate::{
    application::errors::LightningError,
    domains::{
        invoice::Invoice, ln_node::LnEventsUseCases, payment::Payment, system::HealthStatus,
    },
    infra::{config::config_rs::deserialize_duration, lightning::LnClient},
};

use super::{
    lnd_websocket_client::connect_websocket, ErrorResponse, GetinfoRequest, GetinfoResponse,
    InvoiceRequest, InvoiceResponse, ListInvoicesRequest, ListInvoicesResponse, PayRequest,
    PayResponse,
};

#[derive(Clone, Debug, Deserialize)]
pub struct LndRestClientConfig {
    pub endpoint: String,
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
    pub ws_min_reconnect_delay_delay: u64,
    pub ws_max_reconnect_delay_delay: u64,
    pub ca_cert_path: Option<String>,
    pub macaroon_path: String,
}

pub struct LndRestClient {
    client: Client,
    base_url: String,
    maxfeepercent: Option<f64>,
    retry_for: Option<u32>,
    payment_exemptfee: Option<u64>,
    //ws_client: WsClient,
}

const USER_AGENT: &str = "Numeraire Swissknife/1.0";

impl LndRestClient {
    pub async fn new(
        config: LndRestClientConfig,
        ln_events: Arc<dyn LnEventsUseCases>,
    ) -> Result<Self, LightningError> {
        let macaroon = read_macaroon(&config.macaroon_path)
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
            let ca_certificate = read_ca(ca_cert_path)
                .await
                .map_err(|e| LightningError::ReadCertificates(e.to_string()))?;
            client_builder = client_builder
                .add_root_certificate(ca_certificate)
                .danger_accept_invalid_hostnames(config.accept_invalid_hostnames);
        }

        let client = client_builder
            .build()
            .map_err(|e| LightningError::ParseConfig(e.to_string()))?;

        //let ws_client = connect_websocket(&config, macaroon, ln_events).await?;

        Ok(Self {
            client,
            base_url: config.endpoint,
            maxfeepercent: config.maxfeepercent,
            retry_for: Some(config.payment_timeout.as_secs() as u32),
            payment_exemptfee: config.payment_exemptfee,
            // ws_client,
        })
    }

    async fn post_request<T: DeserializeOwned>(
        &self,
        endpoint: &str,
        payload: &impl Serialize,
    ) -> anyhow::Result<T> {
        let response = self
            .client
            .post(format!("{}/v1/{}", self.base_url, endpoint))
            .json(payload)
            .send()
            .await?;

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
        /*self.ws_client
        .disconnect()
        .await
        .map_err(|e| LightningError::Disconnect(e.to_string()))
        */
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

        let response: InvoiceResponse = self
            .post_request("invoices", &request)
            .await
            .map_err(|e| LightningError::Invoice(e.to_string()))?;

        println!("response: {:?}", response);

        let bolt11 = Bolt11Invoice::from_str(&response.payment_request)
            .map_err(|e| LightningError::Invoice(e.to_string()))?;

        let mut invoice: Invoice = bolt11.into();
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
        let response: ListInvoicesResponse = self
            .post_request(
                "listinvoices",
                &ListInvoicesRequest {
                    payment_hash: Some(payment_hash),
                },
            )
            .await
            .map_err(|e| LightningError::InvoiceByHash(e.to_string()))?;

        match response.invoices.into_iter().next() {
            Some(invoice) => Ok(Some(invoice.into())),
            None => Ok(None),
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
        self.post_request::<GetinfoResponse>("getinfo", &GetinfoRequest {})
            .await
            .map_err(|e| LightningError::HealthCheck(e.to_string()))?;

        Ok(HealthStatus::Operational)
    }
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
