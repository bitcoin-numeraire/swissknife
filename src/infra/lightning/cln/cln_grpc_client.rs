use std::{path::PathBuf, str::FromStr, time::Duration};

use breez_sdk_core::ReverseSwapInfo;
use chrono::{TimeZone, Utc};
use hex::decode;
use lightning_invoice::Bolt11Invoice;
use serde::Deserialize;
use tokio::{fs, io};
use tonic::transport::{Certificate, Channel, ClientTlsConfig, Identity};
use uuid::Uuid;

use async_trait::async_trait;
use cln::{
    node_client::NodeClient, Amount, AmountOrAll, Feerate, GetinfoRequest, ListinvoicesRequest, NewaddrRequest,
    PayRequest, WithdrawRequest,
};

use crate::{
    application::{
        entities::Ledger,
        errors::{BitcoinError, LightningError},
    },
    domains::{
        bitcoin::{
            BitcoinOutput, BtcTransaction, BtcTransactionOutput, BitcoinWallet, BtcAddressType, BtcNetwork,
        },
        invoice::Invoice,
        payment::{LnPayment, Payment, PaymentStatus},
        system::HealthStatus,
    },
    infra::{
        config::config_rs::deserialize_duration,
        lightning::{cln::cln::newaddr_request::NewaddrAddresstype, types::parse_network, LnClient},
    },
};

use self::cln::InvoiceRequest;

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
    network: BtcNetwork,
}

impl ClnGrpcClient {
    pub async fn new(config: ClnClientConfig) -> Result<Self, LightningError> {
        let client = Self::connect(&config).await?;

        let mut cln_client = Self {
            client: client.clone(),
            maxfeepercent: config.maxfeepercent,
            retry_for: Some(config.payment_timeout.as_secs() as u32),
            payment_exemptfee: config.payment_exemptfee.map(|fee| Amount { msat: fee }),
            network: BtcNetwork::default(),
        };

        let network = cln_client.network().await?;
        cln_client.network = network;

        Ok(cln_client)
    }

    pub async fn connect(config: &ClnClientConfig) -> Result<NodeClient<Channel>, LightningError> {
        let (identity, ca_certificate) = Self::read_certificates(&config.certs_dir)
            .await
            .map_err(|e| LightningError::ReadCertificates(e.to_string()))?;

        let tls_config = ClientTlsConfig::new()
            .identity(identity)
            .ca_certificate(ca_certificate)
            .domain_name("localhost"); // Use localhost if you are not sure about the domain name

        let endpoint = Channel::from_shared(config.endpoint.clone())
            .map_err(|e| LightningError::ParseConfig(e.to_string()))?
            .tls_config(tls_config)
            .map_err(|e| LightningError::ParseConfig(e.to_string()))?;

        let channel = endpoint
            .connect()
            .await
            .map_err(|e| LightningError::Connect(e.to_string()))?;

        Ok(NodeClient::new(channel))
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

    async fn network(&self) -> Result<BtcNetwork, LightningError> {
        let mut client = self.client.clone();
        let response = client
            .getinfo(GetinfoRequest {})
            .await
            .map_err(|e| LightningError::NodeInfo(e.message().to_string()))?
            .into_inner();

        Ok(parse_network(&response.network))
    }

    fn map_address_type(address_type: BtcAddressType) -> Option<i32> {
        match address_type {
            BtcAddressType::P2tr => Some(NewaddrAddresstype::P2tr as i32),
            _ => Some(NewaddrAddresstype::Bech32 as i32),
        }
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

    async fn pay(&self, bolt11: String, amount_msat: Option<u64>, label: String) -> Result<Payment, LightningError> {
        let mut client = self.client.clone();

        let response = client
            .pay(PayRequest {
                bolt11,
                amount_msat: amount_msat.map(|msat| cln::Amount { msat }),
                label: Some(label),
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

    async fn payment_by_hash(&self, payment_hash: String) -> Result<Option<Payment>, LightningError> {
        let mut client = self.client.clone();

        let response = client
            .list_send_pays(cln::ListsendpaysRequest {
                payment_hash: decode(payment_hash.clone()).ok(),
                ..Default::default()
            })
            .await
            .map_err(|e| LightningError::Pay(e.message().to_string()))?
            .into_inner();

        let entry = response.payments.into_iter().find(|payment| {
            matches!(
                payment.status(),
                cln::listsendpays_payments::ListsendpaysPaymentsStatus::Complete
                    | cln::listsendpays_payments::ListsendpaysPaymentsStatus::Failed
            )
        });

        let Some(entry) = entry else {
            return Ok(None);
        };

        let amount_msat = entry.amount_msat.as_ref().map(|a| a.msat).unwrap_or_default();
        let amount_sent_msat = entry.amount_sent_msat.as_ref().map(|a| a.msat).unwrap_or_default();
        let payment_time = entry.completed_at.or(Some(entry.created_at)).map(|value| {
            let seconds = value as i64;
            let nanos = ((value as f64 - seconds as f64) * 1e9) as u32;
            Utc.timestamp_opt(seconds, nanos).single().unwrap_or_else(Utc::now)
        });

        let status = match entry.status() {
            cln::listsendpays_payments::ListsendpaysPaymentsStatus::Complete => PaymentStatus::Settled,
            cln::listsendpays_payments::ListsendpaysPaymentsStatus::Failed => PaymentStatus::Failed,
            _ => PaymentStatus::Pending,
        };
        let error = if status == PaymentStatus::Failed {
            Some("Payment failed".to_string())
        } else {
            None
        };

        Ok(Some(Payment {
            ledger: Ledger::Lightning,
            status: status.clone(),
            amount_msat: amount_sent_msat,
            fee_msat: Some(amount_sent_msat.saturating_sub(amount_msat)),
            payment_time,
            error,
            lightning: Some(LnPayment {
                payment_hash: Some(payment_hash),
                payment_preimage: entry.payment_preimage.as_ref().map(hex::encode),
                ..Default::default()
            }),
            ..Default::default()
        }))
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
}

#[async_trait]
impl BitcoinWallet for ClnGrpcClient {
    async fn new_address(&self, address_type: BtcAddressType) -> Result<String, BitcoinError> {
        let mut client = self.client.clone();

        let response = client
            .new_addr(NewaddrRequest {
                addresstype: Self::map_address_type(address_type),
            })
            .await
            .map_err(|e| BitcoinError::Address(e.message().to_string()))?
            .into_inner();

        response
            .bech32
            .or(response.p2tr)
            .ok_or_else(|| BitcoinError::Address("No address returned by CLN".to_string()))
    }

    async fn send(&self, address: String, amount_sat: u64, fee_rate: Option<u32>) -> Result<String, BitcoinError> {
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
            .map_err(|e| BitcoinError::Transaction(e.message().to_string()))?
            .into_inner();

        Ok(hex::encode(response.txid))
    }

    async fn get_transaction(&self, txid: &str) -> Result<BtcTransaction, BitcoinError> {
        let mut client = self.client.clone();

        let response = client
            .list_transactions(cln::ListtransactionsRequest {})
            .await
            .map_err(|e| BitcoinError::Transaction(e.message().to_string()))?
            .into_inner();

        let transaction = response
            .transactions
            .into_iter()
            .find(|transaction| hex::encode(&transaction.hash) == txid)
            .ok_or_else(|| BitcoinError::Transaction(format!("Transaction {txid} not found")))?;

        let outputs = transaction
            .outputs
            .into_iter()
            .map(|output| BtcTransactionOutput {
                output_index: output.index,
                address: None,
                amount_sat: (output.amount_msat.map(|a| a.msat).unwrap_or_default() / 1000),
                is_ours: false,
            })
            .collect();

        Ok(BtcTransaction {
            txid: hex::encode(transaction.hash),
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
        let mut client = self.client.clone();

        let response = client
            .list_funds(cln::ListfundsRequest { spent: None })
            .await
            .map_err(|e| BitcoinError::Transaction(e.message().to_string()))?
            .into_inner();

        let output = response.outputs.into_iter().find(|output| {
            let output_txid = hex::encode(&output.txid);
            if output_txid != txid {
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
            txid: hex::encode(output.txid),
            output_index: output.output,
            address: output.address,
            amount_sat: output.amount_msat.map(|a| a.msat).unwrap_or_default() / 1000,
            block_height: output.blockheight.unwrap_or_default(),
            timestamp: None,
            fee_sat: None,
        }))
    }

    fn network(&self) -> BtcNetwork {
        self.network
    }
}
