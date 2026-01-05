use std::{path::PathBuf, str::FromStr, sync::Arc, time::Duration};

use breez_sdk_core::ReverseSwapInfo;
use chrono::Utc;
use lightning_invoice::Bolt11Invoice;
use reqwest::{
    header::{HeaderMap, HeaderValue},
    Certificate, Client,
};
use rust_socketio::asynchronous::Client as WsClient;
use serde::{de::DeserializeOwned, Deserialize, Serialize};
use tokio::fs;
use uuid::Uuid;

use async_trait::async_trait;

use crate::{
    application::{entities::Currency, errors::LightningError},
    domains::{
        bitcoin::{BitcoinBalance, BitcoinOutput},
        invoice::Invoice,
        ln_node::LnEventsUseCases,
        payment::Payment,
        system::HealthStatus,
    },
    infra::{
        config::config_rs::deserialize_duration,
        lightning::{types::currency_from_network_name, types::validate_address_for_currency, LnClient},
    },
};

use super::{
    cln_websocket_client::connect_websocket, ErrorResponse, GetinfoRequest, GetinfoResponse, InvoiceRequest,
    InvoiceResponse, ListFundsRequest, ListFundsResponse, ListInvoicesRequest, ListInvoicesResponse, NewAddrRequest,
    NewAddrResponse, PayRequest, PayResponse, WithdrawRequest, WithdrawResponse,
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
        let mut rune_header =
            HeaderValue::from_str(&config.rune).map_err(|e| LightningError::ParseConfig(e.to_string()))?;
        rune_header.set_sensitive(true);
        headers.insert("Rune", rune_header);

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

    async fn read_ca(path: &str) -> anyhow::Result<Certificate> {
        let ca_file = fs::read(PathBuf::from(path)).await?;
        let ca_certificate = Certificate::from_pem(&ca_file)?;

        Ok(ca_certificate)
    }

    async fn post_request<T: DeserializeOwned>(&self, endpoint: &str, payload: &impl Serialize) -> anyhow::Result<T> {
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

    fn parse_amount_msat(value: &str) -> anyhow::Result<u64> {
        let cleaned = value.trim_end_matches("msat");
        Ok(cleaned.parse::<u64>()?)
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
        deschashonly: bool,
    ) -> Result<Invoice, LightningError> {
        let response: InvoiceResponse = self
            .post_request(
                "invoice",
                &InvoiceRequest {
                    description,
                    expiry: expiry as u64,
                    label: Uuid::new_v4(),
                    amount_msat,
                    deschashonly: Some(deschashonly),
                },
            )
            .await
            .map_err(|e| LightningError::Invoice(e.to_string()))?;

        let bolt11 = Bolt11Invoice::from_str(&response.bolt11).map_err(|e| LightningError::Invoice(e.to_string()))?;

        Ok(bolt11.into())
    }

    async fn pay(&self, bolt11: String, amount_msat: Option<u64>) -> Result<Payment, LightningError> {
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

    async fn invoice_by_hash(&self, payment_hash: String) -> Result<Option<Invoice>, LightningError> {
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
        Err(LightningError::Unsupported(
            "Bitcoin payments are not implemented for CLN REST client".to_string(),
        ))
    }

    async fn health(&self) -> Result<HealthStatus, LightningError> {
        self.post_request::<GetinfoResponse>("getinfo", &GetinfoRequest {})
            .await
            .map_err(|e| LightningError::HealthCheck(e.to_string()))?;

        Ok(HealthStatus::Operational)
    }

    async fn get_new_bitcoin_address(&self) -> Result<String, LightningError> {
        let response: NewAddrResponse = self
            .post_request("newaddr", &NewAddrRequest { addresstype: None })
            .await
            .map_err(|e| LightningError::BitcoinAddress(e.to_string()))?;

        response
            .bech32
            .or(response.p2tr)
            .or(response.p2sh_segwit)
            .ok_or_else(|| LightningError::BitcoinAddress("No address returned by CLN".to_string()))
    }

    async fn get_bitcoin_balance(&self) -> Result<BitcoinBalance, LightningError> {
        let list_funds: ListFundsResponse = self
            .post_request("listfunds", &ListFundsRequest { spent: Some(false) })
            .await
            .map_err(|e| LightningError::BitcoinBalance(e.to_string()))?;

        let mut confirmed_sat = 0;
        let mut unconfirmed_sat = 0;

        for output in list_funds.outputs {
            let amount_msat = Self::parse_amount_msat(&output.amount_msat)
                .map_err(|e| LightningError::BitcoinBalance(e.to_string()))?;
            let amount_sat = amount_msat / 1000;

            match output.status.to_lowercase().as_str() {
                "confirmed" => confirmed_sat += amount_sat,
                _ => unconfirmed_sat += amount_sat,
            }
        }

        Ok(BitcoinBalance {
            confirmed_sat,
            unconfirmed_sat,
        })
    }

    async fn send_bitcoin(
        &self,
        address: String,
        amount_sat: u64,
        fee_rate: Option<u32>,
    ) -> Result<String, LightningError> {
        let response: WithdrawResponse = self
            .post_request(
                "withdraw",
                &WithdrawRequest {
                    destination: address,
                    satoshi: amount_sat,
                    feerate: fee_rate.map(|rate| format!("{rate}perkb")),
                },
            )
            .await
            .map_err(|e| LightningError::BitcoinPayment(e.to_string()))?;

        Ok(response.txid)
    }

    async fn list_bitcoin_outputs(&self) -> Result<Vec<BitcoinOutput>, LightningError> {
        let currency = self.get_bitcoin_network().await?;
        let list_funds: ListFundsResponse = self
            .post_request("listfunds", &ListFundsRequest { spent: Some(false) })
            .await
            .map_err(|e| LightningError::BitcoinOutputs(e.to_string()))?;

        let outputs = list_funds
            .outputs
            .into_iter()
            .map(|output| {
                let amount_msat = Self::parse_amount_msat(&output.amount_msat).unwrap_or_default();

                BitcoinOutput {
                    id: Uuid::new_v4(),
                    outpoint: format!("{}:{}", output.txid, output.output),
                    txid: output.txid.clone(),
                    output_index: output.output,
                    address: output.address,
                    amount_sat: (amount_msat / 1000) as i64,
                    fee_sat: None,
                    block_height: output.blockheight,
                    timestamp: None,
                    currency: currency.clone(),
                    created_at: Utc::now(),
                    updated_at: None,
                }
            })
            .collect();

        Ok(outputs)
    }

    async fn get_bitcoin_network(&self) -> Result<Currency, LightningError> {
        let response: GetinfoResponse = self
            .post_request("getinfo", &GetinfoRequest {})
            .await
            .map_err(|e| LightningError::HealthCheck(e.to_string()))?;

        currency_from_network_name(&response.network)
            .ok_or_else(|| LightningError::HealthCheck("Unknown network returned by CLN".to_string()))
    }

    async fn validate_bitcoin_address(&self, address: &str) -> Result<bool, LightningError> {
        let currency = self.get_bitcoin_network().await?;
        Ok(validate_address_for_currency(address, currency))
    }
}
