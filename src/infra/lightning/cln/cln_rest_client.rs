use std::{path::PathBuf, str::FromStr, time::Duration};

use breez_sdk_core::ReverseSwapInfo;
use chrono::{TimeZone, Utc};
use lightning_invoice::Bolt11Invoice;
use reqwest::{
    header::{HeaderMap, HeaderValue},
    Certificate, Client,
};
use serde::{de::DeserializeOwned, Deserialize, Serialize};
use tokio::fs;
use uuid::Uuid;

use async_trait::async_trait;

use crate::{
    application::{
        entities::Ledger,
        errors::{BitcoinError, LightningError},
    },
    domains::{
        bitcoin::{
            BitcoinOutput, BitcoinTransaction, BitcoinTransactionOutput, BitcoinWallet, BtcAddressType, BtcNetwork,
        },
        invoice::Invoice,
        payment::{LnPayment, Payment, PaymentStatus},
        system::HealthStatus,
    },
    infra::{
        config::config_rs::deserialize_duration,
        lightning::{types::parse_network, LnClient},
    },
};

use super::{
    ErrorResponse, GetinfoRequest, GetinfoResponse, InvoiceRequest, InvoiceResponse, ListFundsRequest,
    ListFundsResponse, ListInvoicesRequest, ListInvoicesResponse, ListPaysRequest, ListPaysResponse,
    ListTransactionsRequest, ListTransactionsResponse, NewAddrRequest, NewAddrResponse, PayRequest, PayResponse,
    WithdrawRequest, WithdrawResponse,
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
    network: BtcNetwork,
}

const USER_AGENT: &str = "Numeraire Swissknife/1.0";

impl ClnRestClient {
    pub async fn new(config: ClnRestClientConfig) -> Result<Self, LightningError> {
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

        let mut cln_client = Self {
            client,
            base_url: config.endpoint.clone(),
            maxfeepercent: config.maxfeepercent,
            retry_for: Some(config.payment_timeout.as_secs() as u32),
            payment_exemptfee: config.payment_exemptfee,
            network: BtcNetwork::default(),
        };

        let network = cln_client.network().await?;
        cln_client.network = network;

        Ok(cln_client)
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

    async fn network(&self) -> Result<BtcNetwork, LightningError> {
        let response: GetinfoResponse = self
            .post_request("getinfo", &GetinfoRequest {})
            .await
            .map_err(|e| LightningError::NodeInfo(e.to_string()))?;

        Ok(parse_network(&response.network))
    }

    fn parse_amount_msat(value: &str) -> anyhow::Result<u64> {
        let cleaned = value.trim_end_matches("msat");
        Ok(cleaned.parse::<u64>()?)
    }

    fn map_address_type(address_type: BtcAddressType) -> Option<String> {
        match address_type {
            BtcAddressType::P2wpkh => Some("bech32".to_string()),
            BtcAddressType::P2tr => Some("p2tr".to_string()),
            _ => None,
        }
    }

    pub async fn list_pays(&self, payment_hash: String) -> Result<ListPaysResponse, LightningError> {
        self.post_request(
            "listpays",
            &ListPaysRequest {
                payment_hash: Some(payment_hash),
            },
        )
        .await
        .map_err(|e| LightningError::Sync(e.to_string()))
    }

    pub async fn list_funds(&self, spent: Option<bool>) -> Result<ListFundsResponse, LightningError> {
        self.post_request("listfunds", &ListFundsRequest { spent })
            .await
            .map_err(|e| LightningError::Sync(e.to_string()))
    }
}

#[async_trait]
impl LnClient for ClnRestClient {
    async fn disconnect(&self) -> Result<(), LightningError> {
        Ok(())
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

    async fn pay(&self, bolt11: String, amount_msat: Option<u64>, label: String) -> Result<Payment, LightningError> {
        let response: PayResponse = self
            .post_request(
                "pay",
                &PayRequest {
                    bolt11,
                    amount_msat,
                    label: Some(label),
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

    async fn payment_by_hash(&self, payment_hash: String) -> Result<Option<Payment>, LightningError> {
        let response = self.list_pays(payment_hash.clone()).await?;
        let payment = response
            .pays
            .into_iter()
            .find(|pay| matches!(pay.status.as_str(), "complete" | "failed") && pay.payment_hash == payment_hash);

        let Some(payment) = payment else {
            return Ok(None);
        };

        let amount_msat = payment
            .amount_msat
            .as_ref()
            .and_then(|amount| amount.trim_end_matches("msat").parse::<u64>().ok())
            .unwrap_or_default();
        let amount_sent_msat = payment
            .amount_sent_msat
            .as_ref()
            .and_then(|amount| amount.trim_end_matches("msat").parse::<u64>().ok())
            .unwrap_or(amount_msat);
        let payment_time = payment.completed_at.or(payment.created_at).map(|timestamp| {
            let seconds = timestamp as i64;
            let nanos = ((timestamp - seconds as f64) * 1e9) as u32;
            Utc.timestamp_opt(seconds, nanos).single().unwrap_or_else(Utc::now)
        });

        let status = if payment.status == "complete" {
            PaymentStatus::Settled
        } else {
            PaymentStatus::Failed
        };

        Ok(Some(Payment {
            ledger: Ledger::Lightning,
            status,
            amount_msat: amount_sent_msat,
            fee_msat: Some(amount_sent_msat.saturating_sub(amount_msat)),
            payment_time,
            error: if payment.status == "failed" {
                Some("Payment failed".to_string())
            } else {
                None
            },
            lightning: Some(LnPayment {
                payment_hash: Some(payment_hash),
                payment_preimage: payment.payment_preimage,
                ..Default::default()
            }),
            ..Default::default()
        }))
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
}

#[async_trait]
impl BitcoinWallet for ClnRestClient {
    async fn new_address(&self, address_type: BtcAddressType) -> Result<String, BitcoinError> {
        let response: NewAddrResponse = self
            .post_request(
                "newaddr",
                &NewAddrRequest {
                    addresstype: Self::map_address_type(address_type),
                },
            )
            .await
            .map_err(|e| BitcoinError::Address(e.to_string()))?;

        response
            .bech32
            .or(response.p2tr)
            .ok_or_else(|| BitcoinError::Address("No address returned by CLN".to_string()))
    }

    async fn send(&self, address: String, amount_sat: u64, fee_rate: Option<u32>) -> Result<String, BitcoinError> {
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
            .map_err(|e| BitcoinError::Transaction(e.to_string()))?;

        Ok(response.txid)
    }

    async fn get_transaction(&self, txid: &str) -> Result<BitcoinTransaction, BitcoinError> {
        let response: ListTransactionsResponse = self
            .post_request("listtransactions", &ListTransactionsRequest {})
            .await
            .map_err(|e| BitcoinError::Transaction(e.to_string()))?;

        let transaction = response
            .transactions
            .into_iter()
            .find(|transaction| transaction.hash == txid)
            .ok_or_else(|| BitcoinError::Transaction(format!("Transaction {txid} not found")))?;

        let outputs = transaction
            .outputs
            .into_iter()
            .map(|output| BitcoinTransactionOutput {
                output_index: output.index,
                address: None,
                amount_sat: Self::parse_amount_msat(&output.amount_msat).unwrap_or_default() / 1000,
                is_ours: false,
            })
            .collect();

        Ok(BitcoinTransaction {
            txid: transaction.hash,
            timestamp: None,
            fee_sat: None,
            block_height: transaction.blockheight,
            outputs,
        })
    }

    async fn get_output(
        &self,
        txid: &str,
        output_index: Option<u32>,
        address: Option<&str>,
    ) -> Result<Option<BitcoinOutput>, BitcoinError> {
        let response = self
            .list_funds(None)
            .await
            .map_err(|e| BitcoinError::Transaction(e.to_string()))?;

        let output = response.outputs.into_iter().find(|output| {
            if output.txid != txid {
                return false;
            }

            if let Some(index) = output_index {
                output.output == index
            } else {
                address
                    .and_then(|target| output.address.as_deref().map(|addr| addr == target))
                    .unwrap_or(false)
            }
        });

        Ok(output.map(|output| BitcoinOutput {
            txid: output.txid,
            output_index: output.output,
            address: output.address,
            amount_sat: Self::parse_amount_msat(&output.amount_msat).unwrap_or_default() / 1000,
            block_height: output.blockheight.unwrap_or_default(),
            timestamp: None,
            fee_sat: None,
        }))
    }

    fn network(&self) -> BtcNetwork {
        self.network
    }
}
