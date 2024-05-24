use std::path::PathBuf;

use breez_sdk_core::{
    LnUrlPayRequestData, LspInformation, NodeState, Payment, ReverseSwapInfo,
    ServiceHealthCheckResponse,
};
use serde::Deserialize;
use tokio::{fs, io};
use tonic::transport::{Certificate, Channel, ClientTlsConfig, Identity};
use uuid::Uuid;

use async_trait::async_trait;
use cln::node_client::NodeClient;

use crate::{
    application::errors::LightningError,
    domains::lightning::entities::{LightningInvoice, LightningPayment},
    infra::lightning::LightningClient,
};

use self::cln::{GetinfoRequest, InvoiceRequest};

pub mod cln {
    tonic::include_proto!("cln");
}

#[derive(Clone, Debug, Deserialize)]
pub struct ClnClientConfig {
    pub endpoint: String,
    pub certs_dir: String,
}

const DEFAULT_CLIENT_CERT_FILENAME: &str = "client.pem";
const DEFAULT_CLIENT_KEY_FILENAME: &str = "client-key.pem";
const DEFAULT_CA_CRT_FILENAME: &str = "ca.pem";

pub struct ClnClient {
    client: NodeClient<Channel>,
}

impl ClnClient {
    pub async fn new(config: ClnClientConfig) -> Result<Self, LightningError> {
        let (identity, ca_certificate) = Self::read_certificates(&config.certs_dir)
            .await
            .map_err(|e| LightningError::ReadCertificates(e.to_string()))?;

        let tls_config = ClientTlsConfig::new()
            .identity(identity)
            .ca_certificate(ca_certificate)
            .domain_name("localhost"); // Use localhost if you are not sure about the domain name

        let channel = Channel::from_shared(config.endpoint)
            .unwrap()
            .tls_config(tls_config)
            .map_err(|e| LightningError::Connect(e.to_string()))?
            .connect()
            .await
            .map_err(|e| LightningError::Connect(e.to_string()))?;

        let mut client = NodeClient::new(channel);
        client
            .getinfo(GetinfoRequest {})
            .await
            .map_err(|e| LightningError::HealthCheck(e.to_string()))?;

        Ok(Self { client })
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
impl LightningClient for ClnClient {
    async fn invoice(
        &self,
        amount_msat: u64,
        description: String,
        expiry: u32,
    ) -> Result<LightningInvoice, LightningError> {
        let mut client = self.client.clone();

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
            .map_err(|e| LightningError::Invoice(e.to_string()))?;

        // TODO: Add warnings from node if necessary for alerting

        let mut invoice: LightningInvoice = response.into_inner().into();
        invoice.label = Some(label);

        Ok(invoice)
    }

    fn node_info(&self) -> Result<NodeState, LightningError> {
        todo!();
    }

    async fn lsp_info(&self) -> Result<LspInformation, LightningError> {
        todo!();
    }

    async fn list_payments(&self) -> Result<Vec<Payment>, LightningError> {
        todo!();
    }

    async fn send_payment(
        &self,
        bolt11: String,
        amount_msat: Option<u64>,
        label: Uuid,
    ) -> Result<LightningPayment, LightningError> {
        todo!();
    }

    async fn send_spontaneous_payment(
        &self,
        node_id: String,
        amount_msat: u64,
        label: Uuid,
    ) -> Result<LightningPayment, LightningError> {
        todo!();
    }

    async fn lnurl_pay(
        &self,
        data: LnUrlPayRequestData,
        amount_msat: u64,
        comment: Option<String>,
        label: Uuid,
    ) -> Result<LightningPayment, LightningError> {
        todo!();
    }

    async fn payment_by_hash(
        &self,
        payment_hash: String,
    ) -> Result<Option<Payment>, LightningError> {
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
