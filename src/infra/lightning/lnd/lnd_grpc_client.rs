use std::{str::FromStr, sync::Arc, time::Duration};

use breez_sdk_core::ReverseSwapInfo;
use lightning_invoice::Bolt11Invoice;
use lnrpc::lightning_client::LightningClient;
use rustls::ClientConfig;
use serde::Deserialize;
use serde_bolt::bitcoin::hashes::{hex::ToHex, sha256, Hash};
use tokio::{fs, io};
use tonic::{
    service::interceptor::InterceptedService,
    transport::{Channel, ClientTlsConfig},
};
use uuid::Uuid;

use async_trait::async_trait;

use crate::{
    application::errors::LightningError,
    domains::{
        invoice::Invoice, ln_node::LnEventsUseCases, payment::Payment, system::HealthStatus,
    },
    infra::{config::config_rs::deserialize_duration, lightning::LnClient},
};

use super::{interceptor::MacaroonInterceptor, tls::AllowCaAsEndEntityVerifier};

pub mod lnrpc {
    tonic::include_proto!("lnrpc");
}

#[derive(Clone, Debug, Deserialize)]
pub struct LndClientConfig {
    pub endpoint: String,
    pub tls_cert_path: String,
    pub macaroon_path: String,
    pub maxfeepercent: Option<f64>,
    #[serde(deserialize_with = "deserialize_duration")]
    pub payment_timeout: Duration,
    pub payment_exemptfee: Option<u64>,
    #[serde(deserialize_with = "deserialize_duration")]
    pub retry_delay: Duration,
}

pub struct LndGrpcClient {
    client: LightningClient<InterceptedService<Channel, MacaroonInterceptor>>,
}

impl LndGrpcClient {
    pub async fn new(
        config: LndClientConfig,
        ln_events: Arc<dyn LnEventsUseCases>,
    ) -> Result<Self, LightningError> {
        let (tls_cert, macaroon_hex) =
            Self::read_credentials(&config.tls_cert_path, &config.macaroon_path)
                .await
                .map_err(|e| LightningError::ReadCertificates(e.to_string()))?;

        let macaroon_interceptor = MacaroonInterceptor::new(&macaroon_hex)?;

        let tls_config = Self::tls_config(tls_cert)?;

        let endpoint = Channel::from_shared(config.endpoint)
            .map_err(|e| LightningError::ParseConfig(e.to_string()))?
            .tls_config(tls_config)
            .map_err(|e| LightningError::ParseConfig(e.to_string()))?;

        let channel = endpoint
            .connect()
            .await
            .map_err(|e| LightningError::Connect(e.to_string()))?;

        let client = LightningClient::with_interceptor(channel, macaroon_interceptor);

        Ok(Self { client })
    }

    async fn read_credentials(
        tls_path: &str,
        macaroon_path: &str,
    ) -> io::Result<(Vec<u8>, String)> {
        let tls_cert = fs::read(tls_path).await?;
        let macaroon_hex = fs::read(macaroon_path).await?.to_hex();

        Ok((tls_cert, macaroon_hex))
    }

    fn tls_config(tls_cert: Vec<u8>) -> Result<ClientConfig, LightningError> {
        let verifier = AllowCaAsEndEntityVerifier::new(tls_cert)?;

        let mut tls_config = ClientConfig::builder()
            .dangerous()
            .with_custom_certificate_verifier(Arc::new(verifier))
            .with_no_client_auth();

        Ok(tls_config.alpn_protocols(vec![b"h2".to_vec()]))
    }
}

#[async_trait]
impl LnClient for LndGrpcClient {
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

        let mut request = lnrpc::Invoice {
            value_msat: amount_msat as i64,
            expiry: expiry as i64,
            memo: description.clone(),
            ..Default::default()
        };

        if deschashonly {
            let description_hash = sha256::Hash::hash(description.as_bytes()).to_vec();
            request.description_hash = description_hash;
        }

        let label = Uuid::new_v4();
        let response = client
            .add_invoice(request)
            .await
            .map_err(|e| LightningError::Invoice(e.message().to_string()))?;

        println!("{:?}", response);

        let bolt11 = Bolt11Invoice::from_str(&response.into_inner().payment_request)
            .map_err(|e| LightningError::Invoice(e.to_string()))?;

        let mut invoice: Invoice = bolt11.into();
        invoice.id = label;

        Ok(invoice)
    }

    async fn pay(
        &self,
        bolt11: String,
        amount_msat: Option<u64>,
    ) -> Result<Payment, LightningError> {
        todo!()
    }

    async fn invoice_by_hash(
        &self,
        payment_hash: String,
    ) -> Result<Option<Invoice>, LightningError> {
        todo!()
    }

    async fn health(&self) -> Result<HealthStatus, LightningError> {
        todo!()
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
