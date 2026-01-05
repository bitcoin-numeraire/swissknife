use std::{path::PathBuf, process, str::FromStr, sync::Arc, time::Duration};

use breez_sdk_core::ReverseSwapInfo;
use chrono::Utc;
use hex::decode;
use lightning_invoice::Bolt11Invoice;
use serde::Deserialize;
use tokio::{fs, io};
use tonic::transport::{Certificate, Channel, ClientTlsConfig, Identity};
use uuid::Uuid;

use async_trait::async_trait;
use cln::{
    node_client::NodeClient, Amount, AmountOrAll, Feerate, GetinfoRequest, ListfundsRequest, ListinvoicesRequest,
    NewaddrRequest, PayRequest, WithdrawRequest,
};

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

use self::cln::InvoiceRequest;

use super::cln_grpc_listener::listen_invoices;

pub mod cln {
    tonic::include_proto!("cln");
}

#[derive(Clone, Debug, Deserialize)]
pub struct ClnClientConfig {
    pub endpoint: String,
    pub certs_dir: String,
    pub maxfeepercent: Option<f64>,
    #[serde(deserialize_with = "deserialize_duration")]
    pub payment_timeout: Duration,
    pub payment_exemptfee: Option<u64>,
    #[serde(deserialize_with = "deserialize_duration")]
    pub retry_delay: Duration,
}

const DEFAULT_CLIENT_CERT_FILENAME: &str = "client.pem";
const DEFAULT_CLIENT_KEY_FILENAME: &str = "client-key.pem";
const DEFAULT_CA_CRT_FILENAME: &str = "ca.pem";

pub struct ClnGrpcClient {
    client: NodeClient<Channel>,
    maxfeepercent: Option<f64>,
    retry_for: Option<u32>,
    payment_exemptfee: Option<Amount>,
}

impl ClnGrpcClient {
    pub async fn new(config: ClnClientConfig, ln_events: Arc<dyn LnEventsUseCases>) -> Result<Self, LightningError> {
        let (identity, ca_certificate) = Self::read_certificates(&config.certs_dir)
            .await
            .map_err(|e| LightningError::ReadCertificates(e.to_string()))?;

        let tls_config = ClientTlsConfig::new()
            .identity(identity)
            .ca_certificate(ca_certificate)
            .domain_name("localhost"); // Use localhost if you are not sure about the domain name

        let endpoint = Channel::from_shared(config.endpoint)
            .map_err(|e| LightningError::ParseConfig(e.to_string()))?
            .tls_config(tls_config)
            .map_err(|e| LightningError::ParseConfig(e.to_string()))?;

        let channel = endpoint
            .connect()
            .await
            .map_err(|e| LightningError::Connect(e.to_string()))?;

        let client = NodeClient::new(channel);

        let listener_client = client.clone();
        tokio::spawn(async move {
            if let Err(err) = listen_invoices(listener_client, ln_events, config.retry_delay)
                .await
                .map_err(|e| LightningError::Listener(e.to_string()))
            {
                tracing::error!(%err, "Event listener failed");
                process::exit(1);
            }
        });

        Ok(Self {
            client,
            maxfeepercent: config.maxfeepercent,
            retry_for: Some(config.payment_timeout.as_secs() as u32),
            payment_exemptfee: config.payment_exemptfee.map(|fee| Amount { msat: fee }),
        })
    }

    async fn read_certificates(certs_dir: &str) -> io::Result<(Identity, Certificate)> {
        let dir = PathBuf::from(certs_dir);

        let key_path = dir.join(DEFAULT_CLIENT_KEY_FILENAME);
        let crt_path = dir.join(DEFAULT_CLIENT_CERT_FILENAME);
        let ca_path = dir.join(DEFAULT_CA_CRT_FILENAME);

        let client_key = fs::read(key_path).await?;
        let client_crt = fs::read(crt_path).await?;
        let ca_cert = fs::read(ca_path).await?;

        let identity = Identity::from_pem(client_crt, client_key);
        let ca_certificate = Certificate::from_pem(ca_cert);

        Ok((identity, ca_certificate))
    }
}

#[async_trait]
impl LnClient for ClnGrpcClient {
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
        let mut client = self.client.clone();

        let response = client
            .invoice(InvoiceRequest {
                description,
                expiry: Some(expiry as u64),
                label: Uuid::new_v4().to_string(),
                deschashonly: Some(deschashonly),
                amount_msat: Some(cln::AmountOrAny {
                    value: Some(cln::amount_or_any::Value::Amount(cln::Amount { msat: amount_msat })),
                }),
                ..Default::default()
            })
            .await
            .map_err(|e| LightningError::Invoice(e.message().to_string()))?;

        let bolt11 = Bolt11Invoice::from_str(&response.into_inner().bolt11)
            .map_err(|e| LightningError::Invoice(e.to_string()))?;

        Ok(bolt11.into())
    }

    async fn pay(&self, bolt11: String, amount_msat: Option<u64>) -> Result<Payment, LightningError> {
        let mut client = self.client.clone();

        let response = client
            .pay(PayRequest {
                bolt11,
                amount_msat: amount_msat.map(|msat| cln::Amount { msat }),
                label: Some(Uuid::new_v4().to_string()),
                maxfeepercent: self.maxfeepercent,
                retry_for: self.retry_for,
                exemptfee: self.payment_exemptfee,
                ..Default::default()
            })
            .await
            .map_err(|e| LightningError::Pay(e.message().to_string()))?
            .into_inner();

        Ok(response.into())
    }

    async fn invoice_by_hash(&self, payment_hash: String) -> Result<Option<Invoice>, LightningError> {
        let mut client = self.client.clone();

        let response = client
            .list_invoices(ListinvoicesRequest {
                payment_hash: decode(payment_hash).ok(),
                ..Default::default()
            })
            .await
            .map_err(|e| LightningError::InvoiceByHash(e.message().to_string()))?
            .into_inner();

        match response.invoices.into_iter().next() {
            Some(invoice) => Ok(Some(invoice.into())),
            None => Ok(None),
        }
    }

    async fn health(&self) -> Result<HealthStatus, LightningError> {
        let mut client = self.client.clone();

        client
            .getinfo(GetinfoRequest {})
            .await
            .map_err(|e| LightningError::HealthCheck(e.message().to_string()))?;

        Ok(HealthStatus::Operational)
    }

    async fn pay_onchain(
        &self,
        _amount_sat: u64,
        _recipient_address: String,
        _feerate: u32,
    ) -> Result<ReverseSwapInfo, LightningError> {
        Err(LightningError::Unsupported(
            "Bitcoin payments are not implemented for CLN gRPC client".to_string(),
        ))
    }

    async fn get_new_bitcoin_address(&self) -> Result<String, LightningError> {
        let mut client = self.client.clone();

        let response = client
            .new_addr(NewaddrRequest { addresstype: None })
            .await
            .map_err(|e| LightningError::BitcoinAddress(e.message().to_string()))?
            .into_inner();

        response
            .bech32
            .or(response.p2tr)
            .ok_or_else(|| LightningError::BitcoinAddress("No address returned by CLN".to_string()))
    }

    async fn get_bitcoin_balance(&self) -> Result<BitcoinBalance, LightningError> {
        let mut client = self.client.clone();

        let response = client
            .list_funds(ListfundsRequest { spent: Some(false) })
            .await
            .map_err(|e| LightningError::BitcoinBalance(e.message().to_string()))?
            .into_inner();

        let mut confirmed_sat = 0;
        let mut unconfirmed_sat = 0;

        for output in response.outputs {
            let amount_msat = output.amount_msat.map(|a| a.msat).unwrap_or_default();
            let amount_sat = amount_msat / 1000;

            match cln::listfunds_outputs::ListfundsOutputsStatus::try_from(output.status) {
                Ok(cln::listfunds_outputs::ListfundsOutputsStatus::Confirmed) => confirmed_sat += amount_sat,
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
        let mut client = self.client.clone();
        let feerate = fee_rate.map(|rate| Feerate {
            style: Some(cln::feerate::Style::Perkb(rate * 1000)),
        });

        let response = client
            .withdraw(WithdrawRequest {
                destination: address,
                satoshi: Some(AmountOrAll {
                    value: Some(cln::amount_or_all::Value::Amount(Amount {
                        msat: amount_sat * 1000,
                    })),
                }),
                minconf: None,
                utxos: vec![],
                feerate,
            })
            .await
            .map_err(|e| LightningError::BitcoinPayment(e.message().to_string()))?
            .into_inner();

        Ok(hex::encode(response.txid))
    }

    async fn list_bitcoin_outputs(&self) -> Result<Vec<BitcoinOutput>, LightningError> {
        let mut client = self.client.clone();
        let currency = self.get_bitcoin_network().await?;

        let response = client
            .list_funds(ListfundsRequest { spent: Some(false) })
            .await
            .map_err(|e| LightningError::BitcoinOutputs(e.message().to_string()))?
            .into_inner();

        let outputs = response
            .outputs
            .into_iter()
            .map(|output| BitcoinOutput {
                id: Uuid::new_v4(),
                outpoint: format!("{}:{}", hex::encode(output.txid.clone()), output.output),
                txid: hex::encode(output.txid),
                output_index: output.output,
                address: output.address,
                amount_sat: (output.amount_msat.map(|a| a.msat).unwrap_or_default() / 1000) as i64,
                fee_sat: None,
                block_height: output.blockheight,
                timestamp: None,
                currency: currency.clone(),
                created_at: Utc::now(),
                updated_at: None,
            })
            .collect();

        Ok(outputs)
    }

    async fn get_bitcoin_network(&self) -> Result<Currency, LightningError> {
        let mut client = self.client.clone();
        let response = client
            .getinfo(GetinfoRequest {})
            .await
            .map_err(|e| LightningError::HealthCheck(e.message().to_string()))?
            .into_inner();

        currency_from_network_name(&response.network)
            .ok_or_else(|| LightningError::HealthCheck("Unknown network returned by CLN".to_string()))
    }

    async fn validate_bitcoin_address(&self, address: &str) -> Result<bool, LightningError> {
        let currency = self.get_bitcoin_network().await?;
        Ok(validate_address_for_currency(address, currency))
    }
}
