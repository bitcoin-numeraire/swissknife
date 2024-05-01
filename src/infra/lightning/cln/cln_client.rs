use breez_sdk_core::{
    LnUrlPayRequestData, LspInformation, NodeState, Payment, ServiceHealthCheckResponse,
};
use serde::Deserialize;
use tonic::transport::Channel;
use uuid::Uuid;

use async_trait::async_trait;
use cln::node_client::NodeClient;

use crate::{
    application::errors::LightningError,
    domains::lightning::entities::{LightningInvoice, LightningPayment},
    infra::lightning::LightningClient,
};

pub mod cln {
    tonic::include_proto!("cln");
}

#[derive(Clone, Debug, Deserialize)]
pub struct ClnClientConfig {
    pub certs_dir: String,
}

const DEFAULT_CLIENT_CERT_FILENAME: &str = "client.crt";
const DEFAULT_CLIENT_KEY_FILENAME: &str = "client-key.pem";

pub struct ClnClient {
    client: NodeClient<Channel>,
}

impl ClnClient {
    pub async fn new(config: ClnClientConfig) -> Result<Self, LightningError> {
        let client = NodeClient::connect("127.0.0.1:11003")
            .await
            .map_err(|e| LightningError::Connect(e.to_string()))?;

        Ok(Self { client })
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
        todo!();
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
}
