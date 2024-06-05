use std::str::FromStr;

use breez_sdk_core::{
    LnUrlPayRequestData, LspInformation, NodeState, Payment as BreezPayment, ReverseSwapInfo,
    ServiceHealthCheckResponse,
};
use humantime::parse_duration;
use lightning_invoice::Bolt11Invoice;
use reqwest::{
    header::{HeaderMap, HeaderValue},
    Client,
};
use serde::{de::DeserializeOwned, Deserialize, Serialize};
use uuid::Uuid;

use async_trait::async_trait;

use crate::{
    application::errors::LightningError,
    domains::{invoices::entities::Invoice, payments::entities::Payment},
    infra::lightning::LnClient,
};

use super::{cln_ws_client::ClnWsClient, InvoiceRequest, InvoiceResponse};

#[derive(Clone, Debug, Deserialize)]
pub struct ClnRestClientConfig {
    pub endpoint: String,
    pub rune: String,
    pub connect_timeout: String,
    pub connection_verbose: bool,
    pub timeout: String,
    pub accept_invalid_certs: bool,
    pub ws_endpoint: String,
}

pub struct ClnRestClient {
    client: Client,
    base_url: String,
}

const USER_AGENT: &str = "Numeraire Swissknife/1.0";

impl ClnRestClient {
    pub async fn new(config: ClnRestClientConfig) -> Result<Self, LightningError> {
        let connect_timeout = parse_duration(&config.connect_timeout)
            .map_err(|e| LightningError::ParseConfig(e.to_string()))?;

        let timeout = parse_duration(&config.connect_timeout)
            .map_err(|e| LightningError::ParseConfig(e.to_string()))?;

        let mut headers = HeaderMap::new();
        let mut rune_header = HeaderValue::from_str(&config.rune)
            .map_err(|e| LightningError::ParseConfig(e.to_string()))?;
        rune_header.set_sensitive(true);
        headers.insert("Rune", rune_header);

        let client = Client::builder()
            .user_agent(USER_AGENT)
            .connect_timeout(connect_timeout)
            .timeout(timeout)
            .connection_verbose(config.connection_verbose)
            .default_headers(headers)
            .danger_accept_invalid_certs(config.accept_invalid_certs)
            .build()
            .map_err(|e| LightningError::ParseConfig(e.to_string()))?;

        let ws_client = ClnWsClient::new(config.clone())?;
        ws_client.listen();

        Ok(Self {
            client,
            base_url: config.endpoint,
        })
    }

    async fn post_request<T: DeserializeOwned>(
        &self,
        endpoint: &str,
        payload: &impl Serialize,
    ) -> Result<T, LightningError> {
        let response = self
            .client
            .post(format!("{}/{}", self.base_url, endpoint))
            .json(payload)
            .send()
            .await
            .map_err(|e| LightningError::PostRequest(e.to_string()))?;

        let response = response
            .error_for_status()
            .map_err(|e| LightningError::RequestFailureStatus(e.to_string()))?;

        let result = response
            .json::<T>()
            .await
            .map_err(|e| LightningError::ParseResponseBody(e.to_string()))?;

        Ok(result)
    }
}

#[async_trait]
impl LnClient for ClnRestClient {
    async fn invoice(
        &self,
        amount_msat: u64,
        description: String,
        expiry: u32,
    ) -> Result<Invoice, LightningError> {
        let label = Uuid::new_v4();
        let invoice: InvoiceResponse = self
            .post_request(
                "invoice",
                &InvoiceRequest {
                    description,
                    expiry: expiry as u64,
                    label,
                    amount_msat,
                },
            )
            .await?;

        let bolt11 = Bolt11Invoice::from_str(&invoice.bolt11)
            .map_err(|e| LightningError::Invoice(e.to_string()))?;

        let mut invoice: Invoice = bolt11.into();
        invoice.label = Some(label);

        Ok(invoice)
    }

    fn node_info(&self) -> Result<NodeState, LightningError> {
        todo!();
    }

    async fn lsp_info(&self) -> Result<LspInformation, LightningError> {
        todo!();
    }

    async fn list_payments(&self) -> Result<Vec<BreezPayment>, LightningError> {
        todo!();
    }

    async fn send_payment(
        &self,
        bolt11: String,
        amount_msat: Option<u64>,
        label: Uuid,
    ) -> Result<Payment, LightningError> {
        todo!();
    }

    async fn send_spontaneous_payment(
        &self,
        node_id: String,
        amount_msat: u64,
        label: Uuid,
    ) -> Result<Payment, LightningError> {
        todo!();
    }

    async fn lnurl_pay(
        &self,
        data: LnUrlPayRequestData,
        amount_msat: u64,
        comment: Option<String>,
        label: Uuid,
    ) -> Result<Payment, LightningError> {
        todo!();
    }

    async fn payment_by_hash(
        &self,
        payment_hash: String,
    ) -> Result<Option<BreezPayment>, LightningError> {
        todo!();
    }

    async fn health(&self) -> Result<ServiceHealthCheckResponse, LightningError> {
        todo!();
    }

    async fn list_lsps(&self) -> Result<Vec<LspInformation>, LightningError> {
        todo!();
    }

    async fn close_lsp_channels(&self) -> Result<Vec<String>, LightningError> {
        todo!();
    }

    async fn pay_onchain(
        &self,
        amount_sat: u64,
        recipient_address: String,
        feerate: u32,
    ) -> Result<ReverseSwapInfo, LightningError> {
        todo!();
    }

    async fn redeem_onchain(
        &self,
        to_address: String,
        feerate: u32,
    ) -> Result<String, LightningError> {
        todo!();
    }
}
