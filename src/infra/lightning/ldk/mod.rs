use std::{collections::HashSet, future::Future, str::FromStr, time::Duration};

use async_trait::async_trait;
use bitcoin::hashes::{sha256, Hash};
use chrono::DateTime;
mod rpc;
use lightning_invoice::Bolt11Invoice;
use rpc::{api, types, LdkRpc};
use serde::Deserialize;
use tokio::{
    fs,
    time::{sleep, timeout, Instant},
};

use crate::{
    application::{composition::Ledger, errors::LightningError},
    domains::{
        bitcoin::BtcNetwork,
        invoice::{Invoice, InvoiceStatus},
        payment::{LnPayment, LnPaymentTarget, Payment, PaymentStatus},
        system::HealthStatus,
    },
    infra::{
        config::config_rs::deserialize_duration,
        lightning::{types::invoice_from_bolt11, LnClient},
    },
};

#[derive(Clone, Debug, Deserialize)]
pub struct LdkServerConfig {
    /// host:port, without a scheme. The transport always uses verified HTTPS.
    pub endpoint: String,
    pub cert_path: String,
    /// LDK's admin.macaroon contains a hex string, unlike LND's binary file.
    pub macaroon_path: String,
    pub fee_limit_msat: u64,
    #[serde(deserialize_with = "deserialize_duration")]
    pub request_timeout: Duration,
    #[serde(deserialize_with = "deserialize_duration")]
    pub payment_timeout: Duration,
    #[serde(deserialize_with = "deserialize_duration")]
    pub poll_interval: Duration,
}

pub struct LdkClient {
    client: LdkRpc,
    config: LdkServerConfig,
    network: BtcNetwork,
}

impl LdkClient {
    pub async fn new(config: LdkServerConfig) -> Result<Self, LightningError> {
        if config.request_timeout.is_zero() || config.payment_timeout.is_zero() || config.poll_interval.is_zero() {
            return Err(LightningError::ParseConfig(
                "LDK timeouts and poll interval must be positive".into(),
            ));
        }
        let (certificate, macaroon) =
            tokio::try_join!(fs::read(&config.cert_path), fs::read_to_string(&config.macaroon_path))
                .map_err(|e| LightningError::ReadCertificates(e.to_string()))?;
        let client = LdkRpc::connect(&config.endpoint, macaroon.trim(), certificate, config.request_timeout).await?;
        let mut result = Self {
            client,
            config,
            network: BtcNetwork::Regtest,
        };
        let info = result
            .rpc(result.client.get_node_info(api::GetNodeInfoRequest {}))
            .await?;
        result.network = match types::Network::try_from(info.network).ok() {
            Some(types::Network::Bitcoin) => BtcNetwork::Bitcoin,
            Some(types::Network::Testnet) => BtcNetwork::Testnet,
            Some(types::Network::Testnet4) => BtcNetwork::Testnet4,
            Some(types::Network::Signet) => BtcNetwork::Signet,
            Some(types::Network::Regtest) => BtcNetwork::Regtest,
            None => return Err(LightningError::NodeInfo("Unknown LDK network".into())),
        };
        Ok(result)
    }

    pub fn network(&self) -> BtcNetwork {
        self.network
    }

    async fn rpc<T>(&self, operation: impl Future<Output = Result<T, tonic::Status>>) -> Result<T, LightningError> {
        timeout(self.config.request_timeout, operation)
            .await
            .map_err(|_| LightningError::Connect("LDK RPC deadline exceeded".into()))?
            .map_err(|e| LightningError::Connect(e.to_string()))
    }

    async fn lookup(
        &self,
        hash: &str,
        direction: types::PaymentDirection,
    ) -> Result<Option<types::Payment>, LightningError> {
        // Payment IDs are opaque: do not send a BOLT11 hash as payment_id.
        // Bound the complete scan, including every page, not just each request.
        timeout(self.config.request_timeout, async {
            let mut token = None;
            let mut seen = HashSet::new();
            let mut found: Option<types::Payment> = None;
            for _ in 0..1000 {
                let page = self
                    .rpc(
                        self.client
                            .list_payments(api::ListPaymentsRequest { page_token: token }),
                    )
                    .await?;
                for payment in page.payments {
                    let bolt11 = match payment.kind.as_ref().and_then(|k| k.kind.as_ref()) {
                        Some(types::payment_kind::Kind::Bolt11(bolt11)) => bolt11,
                        _ => continue,
                    };
                    if payment.direction == direction as i32 && bolt11.hash == hash {
                        // A successful attempt is authoritative; otherwise prefer the
                        // latest record so an older failure cannot end a pending retry.
                        let replace = found.as_ref().is_none_or(|previous| {
                            previous.status != types::PaymentStatus::Succeeded as i32
                                && (payment.status == types::PaymentStatus::Succeeded as i32
                                    || payment.latest_update_timestamp >= previous.latest_update_timestamp)
                        });
                        if replace {
                            found = Some(payment);
                        }
                    }
                }
                match page.next_page_token {
                    None => return Ok(found),
                    Some(next) if !next.is_empty() && seen.insert(next.clone()) => token = Some(next),
                    _ => return Err(LightningError::Listener("Invalid LDK payment pagination".into())),
                }
            }
            Err(LightningError::Listener(
                "LDK payment history page limit exceeded".into(),
            ))
        })
        .await
        .map_err(|_| LightningError::Connect("LDK payment lookup deadline exceeded".into()))?
    }

    fn map_payment(record: types::Payment) -> Result<Payment, LightningError> {
        let Some(types::payment_kind::Kind::Bolt11(bolt11)) = record.kind.and_then(|k| k.kind) else {
            return Err(LightningError::PaymentByHash("Expected a BOLT11 payment".into()));
        };
        let status = match types::PaymentStatus::try_from(record.status).ok() {
            Some(types::PaymentStatus::Pending) => PaymentStatus::Pending,
            Some(types::PaymentStatus::Succeeded) => PaymentStatus::Settled,
            Some(types::PaymentStatus::Failed) => PaymentStatus::Failed,
            None => return Err(LightningError::PaymentByHash("Unknown LDK payment status".into())),
        };
        if status == PaymentStatus::Settled {
            let preimage = bolt11.preimage.as_ref().and_then(|s| hex::decode(s).ok());
            if !preimage
                .as_ref()
                .is_some_and(|bytes| bytes.len() == 32 && sha256::Hash::hash(bytes).to_string() == bolt11.hash)
            {
                return Err(LightningError::PaymentByHash(
                    "LDK payment proof does not match its hash".into(),
                ));
            }
            if record.fee_paid_msat.is_none() {
                return Err(LightningError::PaymentByHash(
                    "Settled LDK payment is missing its actual fee".into(),
                ));
            }
        }
        let amount_msat = record
            .amount_msat
            .ok_or_else(|| LightningError::PaymentByHash("Missing LDK payment amount".into()))?;
        let timestamp = i64::try_from(record.latest_update_timestamp)
            .ok()
            .and_then(|s| DateTime::from_timestamp(s, 0))
            .ok_or_else(|| LightningError::PaymentByHash("Invalid LDK payment timestamp".into()))?;
        Ok(Payment {
            amount_msat,
            fee_msat: record.fee_paid_msat,
            status,
            ledger: Ledger::Lightning,
            payment_time: Some(timestamp),
            lightning: Some(LnPayment {
                payment_hash: bolt11.hash,
                payment_preimage: bolt11.preimage,
                ..Default::default()
            }),
            ..Default::default()
        })
    }
}

#[async_trait]
impl LnClient for LdkClient {
    async fn disconnect(&self) -> Result<(), LightningError> {
        Ok(())
    }

    async fn invoice(
        &self,
        amount_msat: u64,
        description: String,
        _label: String,
        expiry: Option<u32>,
        deschashonly: bool,
    ) -> Result<Invoice, LightningError> {
        let expiry = expiry.ok_or(LightningError::Unsupported(
            "LDK Server requires an explicit invoice expiry",
        ))?;
        let kind = if deschashonly {
            types::bolt11_invoice_description::Kind::Hash(sha256::Hash::hash(description.as_bytes()).to_string())
        } else {
            types::bolt11_invoice_description::Kind::Direct(description)
        };
        let response = self
            .rpc(self.client.bolt11_receive(api::Bolt11ReceiveRequest {
                amount_msat: (amount_msat != 0).then_some(amount_msat),
                description: Some(types::Bolt11InvoiceDescription { kind: Some(kind) }),
                expiry_secs: expiry,
            }))
            .await
            .map_err(|e| LightningError::Invoice(e.to_string()))?;
        let invoice = Bolt11Invoice::from_str(&response.invoice).map_err(|e| LightningError::Invoice(e.to_string()))?;
        if invoice.payment_hash().to_string() != response.payment_hash
            || invoice.amount_milli_satoshis() != (amount_msat != 0).then_some(amount_msat)
        {
            return Err(LightningError::Invoice("LDK returned a mismatching invoice".into()));
        }
        Ok(invoice_from_bolt11(invoice))
    }

    async fn estimate_fee(&self, _target: LnPaymentTarget) -> Result<u64, LightningError> {
        Err(LightningError::EstimateFee(
            "LDK Server has no route-estimation RPC; only the configured fee cap is available".into(),
        ))
    }
    fn fee_limit_msat(&self, _amount_msat: u64) -> u64 {
        self.config.fee_limit_msat
    }

    async fn pay(
        &self,
        bolt11: String,
        amount_msat: Option<u64>,
        fee_limit_msat: u64,
        _label: String,
    ) -> Result<Payment, LightningError> {
        let parsed = Bolt11Invoice::from_str(&bolt11).map_err(|e| LightningError::Pay(e.to_string()))?;
        let hash = parsed.payment_hash().to_string();
        // Never retry a send RPC. A timeout or server error may follow dispatch.
        let submitted = timeout(
            self.config.request_timeout,
            self.client.bolt11_send(api::Bolt11SendRequest {
                invoice: bolt11,
                amount_msat,
                route_parameters: Some(types::RouteParametersConfig {
                    max_total_routing_fee_msat: Some(fee_limit_msat.min(self.config.fee_limit_msat)),
                    max_total_cltv_expiry_delta: 1008,
                    max_path_count: 10,
                    max_channel_saturation_power_of_half: 2,
                }),
            }),
        )
        .await;
        if let Ok(Err(status)) = &submitted {
            if matches!(
                status.code(),
                tonic::Code::Unauthenticated | tonic::Code::PermissionDenied | tonic::Code::InvalidArgument
            ) {
                return Err(LightningError::Pay(status.message().to_owned()));
            }
        }
        let deadline = Instant::now() + self.config.payment_timeout;
        loop {
            match tokio::time::timeout_at(deadline, self.payment_by_hash(hash.clone())).await {
                Ok(Ok(Some(payment))) if payment.status == PaymentStatus::Settled => return Ok(payment),
                Ok(Ok(Some(payment))) if payment.status == PaymentStatus::Failed => {
                    return Err(LightningError::Pay("LDK recorded a failed payment".into()))
                }
                _ => {}
            }
            if !matches!(&submitted, Ok(Ok(_))) || Instant::now() >= deadline {
                return Err(LightningError::PaymentOutcomeUnknown(format!(
                    "Reconcile LDK payment {hash} from durable history"
                )));
            }
            sleep(self.config.poll_interval).await;
        }
    }

    async fn invoice_by_hash(&self, hash: String) -> Result<Option<Invoice>, LightningError> {
        let Some(record) = self.lookup(&hash, types::PaymentDirection::Inbound).await? else {
            return Ok(None);
        };
        let status = match types::PaymentStatus::try_from(record.status).ok() {
            Some(types::PaymentStatus::Succeeded) => InvoiceStatus::Settled,
            Some(types::PaymentStatus::Failed) => InvoiceStatus::Expired,
            Some(types::PaymentStatus::Pending) => InvoiceStatus::Pending,
            None => return Err(LightningError::InvoiceByHash("Unknown LDK invoice status".into())),
        };
        let time = i64::try_from(record.latest_update_timestamp)
            .ok()
            .and_then(|s| DateTime::from_timestamp(s, 0))
            .ok_or_else(|| LightningError::InvoiceByHash("Invalid LDK invoice timestamp".into()))?;
        if status == InvoiceStatus::Settled && record.amount_msat.is_none() {
            return Err(LightningError::InvoiceByHash(
                "Settled LDK invoice has no amount".into(),
            ));
        }
        Ok(Some(Invoice {
            status,
            amount_received_msat: record.amount_msat,
            payment_time: Some(time),
            fee_msat: Some(0),
            ..Default::default()
        }))
    }

    async fn payment_by_hash(&self, hash: String) -> Result<Option<Payment>, LightningError> {
        self.lookup(&hash, types::PaymentDirection::Outbound)
            .await?
            .map(Self::map_payment)
            .transpose()
    }

    async fn cancel_invoice(&self, _payment_hash: String, _label: String) -> Result<(), LightningError> {
        Err(LightningError::CancelInvoice(
            "LDK Server cannot cancel ordinary auto-claim invoices".into(),
        ))
    }

    async fn health(&self) -> Result<HealthStatus, LightningError> {
        self.rpc(self.client.get_node_info(api::GetNodeInfoRequest {})).await?;
        Ok(HealthStatus::Operational)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    mod map_payment {
        use super::*;

        fn settled() -> types::Payment {
            let preimage = [7; 32];
            types::Payment {
                kind: Some(types::PaymentKind {
                    kind: Some(types::payment_kind::Kind::Bolt11(types::Bolt11 {
                        hash: sha256::Hash::hash(&preimage).to_string(),
                        preimage: Some(hex::encode(preimage)),
                        ..Default::default()
                    })),
                }),
                amount_msat: Some(21_123),
                fee_paid_msat: Some(1000),
                status: types::PaymentStatus::Succeeded.into(),
                latest_update_timestamp: 1_793_192_000,
                ..Default::default()
            }
        }

        mod with_a_verified_settlement {
            use super::*;
            #[test]
            fn preserves_exact_amount_and_actual_fee() {
                let result = LdkClient::map_payment(settled()).unwrap();
                assert_eq!(result.status, PaymentStatus::Settled);
                assert_eq!(result.amount_msat, 21_123);
                assert_eq!(result.fee_msat, Some(1000));
            }
        }

        mod with_missing_or_invalid_settlement_evidence {
            use super::*;
            #[test]
            fn refuses_to_settle_without_the_actual_fee() {
                let mut record = settled();
                record.fee_paid_msat = None;
                assert!(LdkClient::map_payment(record).is_err());
            }

            #[test]
            fn refuses_a_preimage_for_another_hash() {
                let mut record = settled();
                let Some(types::payment_kind::Kind::Bolt11(bolt11)) =
                    record.kind.as_mut().and_then(|k| k.kind.as_mut())
                else {
                    panic!("BOLT11 fixture");
                };
                bolt11.preimage = Some(hex::encode([8; 32]));
                assert!(LdkClient::map_payment(record).is_err());
            }

            #[test]
            fn refuses_unknown_status_and_overflowing_time() {
                let mut record = settled();
                record.status = 99;
                assert!(LdkClient::map_payment(record).is_err());
                let mut record = settled();
                record.latest_update_timestamp = u64::MAX;
                assert!(LdkClient::map_payment(record).is_err());
            }
        }
    }
}
