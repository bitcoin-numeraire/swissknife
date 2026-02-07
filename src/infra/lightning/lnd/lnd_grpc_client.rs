use std::{str::FromStr, time::Duration};

use async_trait::async_trait;
use chrono::{TimeZone, Utc};
use lightning_invoice::Bolt11Invoice;
use serde::Deserialize;
use serde_bolt::bitcoin::hashes::{sha256, Hash};
use tokio::time::timeout;
use tonic_lnd::{connect, lnrpc, tonic::Code, LightningClient};

use crate::{
    application::{entities::Ledger, errors::LightningError},
    domains::{
        bitcoin::BtcNetwork,
        invoice::{Invoice, InvoiceStatus},
        payment::{LnPayment, Payment, PaymentStatus},
        system::HealthStatus,
    },
    infra::{
        config::config_rs::deserialize_duration,
        lightning::{types::parse_network, LnClient},
    },
};

#[derive(Clone, Debug, Deserialize)]
pub struct LndGrpcClientConfig {
    pub endpoint: String,
    pub cert_path: String,
    pub macaroon_path: String,
    pub fee_limit_msat: u64,
    #[serde(deserialize_with = "deserialize_duration")]
    pub payment_timeout: Duration,
    #[serde(deserialize_with = "deserialize_duration")]
    pub retry_delay: Duration,
}

pub struct LndGrpcClient {
    client: LightningClient,
    fee_limit_msat: u64,
    payment_timeout: Duration,
    network: BtcNetwork,
}

impl LndGrpcClient {
    pub async fn new(config: LndGrpcClientConfig) -> Result<Self, LightningError> {
        let mut client = connect(config.endpoint.clone(), &config.cert_path, &config.macaroon_path)
            .await
            .map_err(|e| LightningError::Connect(e.to_string()))?;

        let lightning_client = client.lightning().clone();

        let mut lnd_client = Self {
            client: lightning_client,
            fee_limit_msat: config.fee_limit_msat,
            payment_timeout: config.payment_timeout,
            network: BtcNetwork::default(),
        };

        let network = lnd_client.network().await?;
        lnd_client.network = network;

        Ok(lnd_client)
    }

    async fn network(&self) -> Result<BtcNetwork, LightningError> {
        let mut client = self.client.clone();
        let response = client
            .get_info(lnrpc::GetInfoRequest {})
            .await
            .map_err(|e| LightningError::NodeInfo(e.message().to_string()))?
            .into_inner();

        if let Some(chain) = response.chains.first() {
            return Ok(parse_network(&chain.network));
        }

        Err(LightningError::NodeInfo(
            "No chain information returned by LND".to_string(),
        ))
    }

    fn invoice_from_lnrpc(&self, response: lnrpc::Invoice) -> Result<Invoice, LightningError> {
        let bolt11 =
            Bolt11Invoice::from_str(&response.payment_request).map_err(|e| LightningError::Invoice(e.to_string()))?;
        let mut invoice: Invoice = bolt11.into();

        match response.state() {
            lnrpc::invoice::InvoiceState::Settled => {
                invoice.status = InvoiceStatus::Settled;
                invoice.payment_time = Some(Utc.timestamp_opt(response.settle_date, 0).unwrap());
                if response.amt_paid_msat > 0 {
                    invoice.amount_received_msat = Some(response.amt_paid_msat as u64);
                }
            }
            lnrpc::invoice::InvoiceState::Open | lnrpc::invoice::InvoiceState::Accepted => {
                invoice.status = InvoiceStatus::Pending;
            }
            lnrpc::invoice::InvoiceState::Canceled => {
                invoice.status = InvoiceStatus::Expired;
            }
        }

        Ok(invoice)
    }

    fn payment_from_lnrpc(&self, payment: lnrpc::Payment) -> Payment {
        let status = match payment.status() {
            lnrpc::payment::PaymentStatus::Succeeded => PaymentStatus::Settled,
            lnrpc::payment::PaymentStatus::Failed => PaymentStatus::Failed,
            lnrpc::payment::PaymentStatus::InFlight => PaymentStatus::Pending,
            lnrpc::payment::PaymentStatus::Unknown => PaymentStatus::Pending,
        };

        let error = match payment.failure_reason() {
            lnrpc::PaymentFailureReason::FailureReasonNone => None,
            reason => Some(format!("{:?}", reason)),
        };

        Payment {
            ledger: Ledger::Lightning,
            status,
            amount_msat: payment.value_msat as u64,
            fee_msat: if payment.fee_msat > 0 {
                Some(payment.fee_msat as u64)
            } else {
                None
            },
            payment_time: Some(Utc.timestamp_nanos(payment.creation_time_ns)),
            error,
            lightning: Some(LnPayment {
                payment_hash: payment.payment_hash,
                payment_preimage: if payment.payment_preimage.is_empty() {
                    None
                } else {
                    Some(payment.payment_preimage)
                },
                ..Default::default()
            }),
            ..Default::default()
        }
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
        let mut request = lnrpc::Invoice {
            memo: description.clone(),
            expiry: expiry as i64,
            value_msat: amount_msat as i64,
            ..Default::default()
        };

        if deschashonly {
            let hash = sha256::Hash::hash(description.as_bytes()).into_inner().to_vec();
            request.description_hash = hash;
        }

        let mut client = self.client.clone();
        let response = client
            .add_invoice(request)
            .await
            .map_err(|e| LightningError::Invoice(e.message().to_string()))?
            .into_inner();

        let bolt11 =
            Bolt11Invoice::from_str(&response.payment_request).map_err(|e| LightningError::Invoice(e.to_string()))?;
        Ok(bolt11.into())
    }

    async fn pay(&self, bolt11: String, amount_msat: Option<u64>, _label: String) -> Result<Payment, LightningError> {
        let request = lnrpc::SendRequest {
            payment_request: bolt11,
            amt_msat: amount_msat.map(|v| v as i64).unwrap_or_default(),
            fee_limit: Some(lnrpc::FeeLimit {
                limit: Some(lnrpc::fee_limit::Limit::FixedMsat(self.fee_limit_msat as i64)),
            }),
            ..Default::default()
        };

        let mut client = self.client.clone();
        let response = timeout(self.payment_timeout, client.send_payment_sync(request))
            .await
            .map_err(|_| LightningError::Pay("Payment timed out".to_string()))?
            .map_err(|e| LightningError::Pay(e.message().to_string()))?
            .into_inner();

        if !response.payment_error.is_empty() {
            return Err(LightningError::Pay(response.payment_error));
        }

        let route = response.payment_route;
        Ok(Payment {
            ledger: Ledger::Lightning,
            status: PaymentStatus::Settled,
            amount_msat: route.as_ref().map(|r| r.total_amt_msat as u64).unwrap_or_default(),
            fee_msat: route
                .as_ref()
                .map(|r| r.total_fees_msat)
                .filter(|fee| *fee > 0)
                .map(|fee| fee as u64),
            lightning: Some(LnPayment {
                payment_hash: hex::encode(response.payment_hash),
                payment_preimage: if response.payment_preimage.is_empty() {
                    None
                } else {
                    Some(hex::encode(response.payment_preimage))
                },
                ..Default::default()
            }),
            ..Default::default()
        })
    }

    async fn invoice_by_hash(&self, payment_hash: String) -> Result<Option<Invoice>, LightningError> {
        let hash_bytes = hex::decode(payment_hash).map_err(|e| LightningError::InvoiceByHash(e.to_string()))?;
        let request = lnrpc::PaymentHash {
            r_hash: hash_bytes,
            ..Default::default()
        };

        let mut client = self.client.clone();
        let response = client.lookup_invoice(request).await;

        match response {
            Ok(response) => Ok(Some(self.invoice_from_lnrpc(response.into_inner())?)),
            Err(err) => match err.code() {
                Code::NotFound => Ok(None),
                _ => Err(LightningError::InvoiceByHash(err.message().to_string())),
            },
        }
    }

    async fn payment_by_hash(&self, payment_hash: String) -> Result<Option<Payment>, LightningError> {
        let mut client = self.client.clone();
        let response = client
            .list_payments(lnrpc::ListPaymentsRequest {
                include_incomplete: true,
                ..Default::default()
            })
            .await
            .map_err(|e| LightningError::PaymentByHash(e.message().to_string()))?
            .into_inner();

        let payment = response
            .payments
            .into_iter()
            .find(|payment| payment.payment_hash == payment_hash);

        Ok(payment.map(|payment| self.payment_from_lnrpc(payment)))
    }

    async fn health(&self) -> Result<HealthStatus, LightningError> {
        let mut client = self.client.clone();
        client
            .get_info(lnrpc::GetInfoRequest {})
            .await
            .map_err(|e| LightningError::HealthCheck(e.message().to_string()))?;

        Ok(HealthStatus::Operational)
    }
}
