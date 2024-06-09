use std::{path::PathBuf, str::FromStr, sync::Arc, time::Duration};

use breez_sdk_core::{
    LspInformation, NodeState, Payment as BreezPayment, ReverseSwapInfo, ServiceHealthCheckResponse,
};
use lightning_invoice::Bolt11Invoice;
use serde::Deserialize;
use tokio::{fs, io};
use tonic::transport::{Certificate, Channel, ClientTlsConfig, Identity};
use uuid::Uuid;

use async_trait::async_trait;
use cln::{node_client::NodeClient, Amount, PayRequest};

use crate::{
    application::errors::LightningError,
    domains::{
        invoices::entities::Invoice, lightning::services::LnEventsUseCases,
        payments::entities::Payment,
    },
    infra::{config::config_rs::deserialize_duration, lightning::LnClient},
};

use self::cln::InvoiceRequest;

use super::cln_grpc_listener::ClnGrpcListener;

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
    listener: ClnGrpcListener,
}

impl ClnGrpcClient {
    pub async fn new(
        config: ClnClientConfig,
        ln_events: Arc<dyn LnEventsUseCases>,
    ) -> Result<Self, LightningError> {
        let (identity, ca_certificate) = Self::read_certificates(&config.certs_dir)
            .await
            .map_err(|e| LightningError::ReadCertificates(e.to_string()))?;

        let tls_config = ClientTlsConfig::new()
            .identity(identity)
            .ca_certificate(ca_certificate)
            .domain_name("localhost"); // Use localhost if you are not sure about the domain name

        let tls_config = Channel::from_shared(config.endpoint)
            .map_err(|e| LightningError::ParseConfig(e.to_string()))?
            .tls_config(tls_config)
            .map_err(|e| LightningError::ParseConfig(e.to_string()))?;

        let channel = tls_config
            .connect()
            .await
            .map_err(|e| LightningError::Connect(e.to_string()))?;

        let client = NodeClient::new(channel);

        let listener = ClnGrpcListener::new(ln_events, config.retry_delay);

        Ok(Self {
            client,
            maxfeepercent: config.maxfeepercent,
            retry_for: Some(config.payment_timeout.as_secs() as u32),
            payment_exemptfee: config.payment_exemptfee.map(|fee| Amount { msat: fee }),
            listener,
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
        let ca_certificate: Certificate = Certificate::from_pem(ca_cert);

        Ok((identity, ca_certificate))
    }
}

#[async_trait]
impl LnClient for ClnGrpcClient {
    async fn listen_events(&self) -> Result<(), LightningError> {
        self.listener
            .clone()
            .listen_invoices(self.client.clone())
            .await
            .map_err(|e| LightningError::Listener(e.to_string()))
    }

    async fn invoice(
        &self,
        amount_msat: u64,
        description: String,
        expiry: u32,
    ) -> Result<Invoice, LightningError> {
        let mut client: NodeClient<Channel> = self.client.clone();

        let label = Uuid::new_v4();
        let response = client
            .invoice(InvoiceRequest {
                description,
                expiry: Some(expiry as u64),
                label: label.to_string(),
                deschashonly: Some(false),
                amount_msat: Some(cln::AmountOrAny {
                    value: Some(cln::amount_or_any::Value::Amount(cln::Amount {
                        msat: amount_msat,
                    })),
                }),
                ..Default::default()
            })
            .await
            .map_err(|e| LightningError::Invoice(e.message().to_string()))?;

        let bolt11 = Bolt11Invoice::from_str(&response.into_inner().bolt11)
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

    async fn pay(
        &self,
        bolt11: String,
        amount_msat: Option<u64>,
        label: Uuid,
    ) -> Result<Payment, LightningError> {
        let mut client: NodeClient<Channel> = self.client.clone();

        let response = client
            .pay(PayRequest {
                bolt11,
                amount_msat: amount_msat.map(|msat| cln::Amount { msat }),
                label: Some(label.to_string()),
                maxfeepercent: self.maxfeepercent,
                retry_for: self.retry_for,
                exemptfee: self.payment_exemptfee.clone(),
                ..Default::default()
            })
            .await
            .map_err(|e| LightningError::Pay(e.message().to_string()))?
            .into_inner();

        Ok(response.into())
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
