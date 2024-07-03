use std::{str::FromStr, sync::Arc, time::Duration};

use breez_sdk_core::{ReverseSwapInfo, ServiceHealthCheckResponse};
use lightning_invoice::Bolt11Invoice;
use reqwest::{
    header::{HeaderMap, HeaderValue},
    Client,
};
use rust_socketio::asynchronous::Client as WsClient;
use serde::{de::DeserializeOwned, Deserialize, Serialize};
use uuid::Uuid;

use async_trait::async_trait;

use crate::{
    application::errors::LightningError,
    domains::{
        invoices::entities::Invoice, lightning::services::LnEventsUseCases,
        payments::entities::Payment,
    },
    infra::{config::config_rs::deserialize_duration, lightning::LnClient},
};

use super::{
    cln_websocket_client::connect_websocket, ErrorResponse, InvoiceRequest, InvoiceResponse,
    ListInvoicesRequest, ListInvoicesResponse, PayRequest, PayResponse,
};

#[derive(Clone, Debug, Deserialize)]
pub struct ClnRestClientConfig {
    pub endpoint: String,
    pub rune: String,
    #[serde(deserialize_with = "deserialize_duration")]
    pub connect_timeout: Duration,
    pub connection_verbose: bool,
    #[serde(deserialize_with = "deserialize_duration")]
    pub timeout: Duration,
    pub accept_invalid_certs: bool,
    pub maxfeepercent: Option<f64>,
    #[serde(deserialize_with = "deserialize_duration")]
    pub payment_timeout: Duration,
    pub payment_exemptfee: Option<u64>,
    pub ws_min_reconnect_delay_delay: u64,
    pub ws_max_reconnect_delay_delay: u64,
}

pub struct ClnRestClient {
    client: Client,
    base_url: String,
    maxfeepercent: Option<f64>,
    retry_for: Option<u32>,
    payment_exemptfee: Option<u64>,
    ws_client: WsClient,
}

const USER_AGENT: &str = "Numeraire Swissknife/1.0";

impl ClnRestClient {
    pub async fn new(
        config: ClnRestClientConfig,
        ln_events: Arc<dyn LnEventsUseCases>,
    ) -> Result<Self, LightningError> {
        let mut headers = HeaderMap::new();
        let mut rune_header = HeaderValue::from_str(&config.rune)
            .map_err(|e| LightningError::ParseConfig(e.to_string()))?;
        rune_header.set_sensitive(true);
        headers.insert("Rune", rune_header);

        let client = Client::builder()
            .user_agent(USER_AGENT)
            .connect_timeout(config.connect_timeout)
            .timeout(config.timeout)
            .connection_verbose(config.connection_verbose)
            .default_headers(headers)
            .danger_accept_invalid_certs(config.accept_invalid_certs)
            .build()
            .map_err(|e| LightningError::ParseConfig(e.to_string()))?;

        let ws_client = connect_websocket(&config, ln_events).await?;

        Ok(Self {
            client,
            base_url: config.endpoint,
            maxfeepercent: config.maxfeepercent,
            retry_for: Some(config.payment_timeout.as_secs() as u32),
            payment_exemptfee: config.payment_exemptfee,
            ws_client,
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
impl LnClient for ClnRestClient {
    async fn disconnect(&self) -> Result<(), LightningError> {
        self.ws_client
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
        let response: InvoiceResponse = self
            .post_request(
                "invoice",
                &InvoiceRequest {
                    description,
                    expiry: expiry as u64,
                    label: Uuid::new_v4(),
                    amount_msat,
                },
            )
            .await
            .map_err(|e| LightningError::Invoice(e.to_string()))?;

        let bolt11 = Bolt11Invoice::from_str(&response.bolt11)
            .map_err(|e| LightningError::Invoice(e.to_string()))?;

        Ok(bolt11.into())
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

    async fn health(&self) -> Result<ServiceHealthCheckResponse, LightningError> {
        todo!();
    }

    async fn pay_onchain(
        &self,
        _amount_sat: u64,
        _recipient_address: String,
        _feerate: u32,
    ) -> Result<ReverseSwapInfo, LightningError> {
        todo!();
    }
}
