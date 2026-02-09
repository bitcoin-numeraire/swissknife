use std::{error::Error as StdError, path::PathBuf, str::FromStr, time::Duration};

use async_trait::async_trait;
use chrono::{TimeZone, Utc};
use lightning_invoice::Bolt11Invoice;
use serde::Deserialize;
use serde_bolt::bitcoin::hashes::{sha256, Hash};
use tokio::{fs, time::timeout};
use tonic::{
    service::{interceptor::InterceptedService, Interceptor},
    transport::{Certificate, Channel, ClientTlsConfig},
    Code, Request, Status,
};

use invoicesrpc::invoices_client::InvoicesClient;
use lnrpc::lightning_client::LightningClient;
use routerrpc::router_client::RouterClient;

use crate::{
    application::{
        entities::Ledger,
        errors::{BitcoinError, LightningError},
    },
    domains::{
        bitcoin::{
            BitcoinWallet, BtcAddressType, BtcNetwork, BtcOutput, BtcPreparedTransaction, BtcTransaction,
            OnchainSyncBatch, OnchainSyncCursor,
        },
        invoice::{Invoice, InvoiceStatus},
        payment::{LnPayment, Payment, PaymentStatus},
        system::HealthStatus,
    },
    infra::{
        config::config_rs::deserialize_duration,
        lightning::{
<<<<<<< ours
            LnClient, bitcoin_utils::parse_psbt, lnd::{
                invoicesrpc::CancelInvoiceMsg,
                lnrpc::{
                    DelCanceledInvoiceReq, GetInfoRequest, NewAddressRequest, PaymentFailureReason, PaymentHash, invoice::InvoiceState
                },
                routerrpc::{SendPaymentRequest, TrackPaymentRequest}, walletrpc::{FinalizePsbtRequest, GetTransactionRequest, TxTemplate, fund_psbt_request},
            }, types::parse_network
=======
            lnd::lnrpc::{invoice::InvoiceState, PaymentFailureReason},
            types::parse_network,
            LnClient,
>>>>>>> theirs
        },
    },
};

use super::lnd_rest_client::read_macaroon;

#[allow(dead_code, clippy::all)]
pub mod lnrpc {
    tonic::include_proto!("lnrpc");
}

#[allow(dead_code, clippy::all)]
pub mod routerrpc {
    tonic::include_proto!("routerrpc");
}

<<<<<<< ours
#[allow(dead_code, clippy::all)]
pub mod invoicesrpc {
    tonic::include_proto!("invoicesrpc");
}

pub mod walletrpc {
    tonic::include_proto!("walletrpc");
}

#[allow(dead_code, clippy::all)]
pub mod signrpc {
    tonic::include_proto!("signrpc");
}

=======
>>>>>>> theirs
#[derive(Clone)]
pub(crate) struct MacaroonInterceptor {
    macaroon: tonic::metadata::AsciiMetadataValue,
}

impl Interceptor for MacaroonInterceptor {
    fn call(&mut self, mut request: Request<()>) -> Result<Request<()>, Status> {
        request.metadata_mut().insert("macaroon", self.macaroon.clone());
        Ok(request)
    }
}

pub(crate) type LndChannel = InterceptedService<Channel, MacaroonInterceptor>;

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
    pub reorg_buffer_blocks: u32,
}

pub struct LndGrpcClient {
    client: LightningClient<LndChannel>,
    invoices: InvoicesClient<LndChannel>,
    router: RouterClient<LndChannel>,
    fee_limit_msat: u64,
    payment_timeout: Duration,
    network: BtcNetwork,
}

impl LndGrpcClient {
    pub async fn new(config: LndGrpcClientConfig) -> Result<Self, LightningError> {
        let channel = Self::connect(&config).await?;

        let mut lnd_client = Self {
            client: LightningClient::new(channel.clone()),
<<<<<<< ours
            invoices: InvoicesClient::new(channel.clone()),
            router: RouterClient::new(channel.clone()),
            wallet: WalletKitClient::new(channel),
=======
            router: RouterClient::new(channel),
>>>>>>> theirs
            fee_limit_msat: config.fee_limit_msat,
            payment_timeout: config.payment_timeout,
            network: BtcNetwork::default(),
        };

        let network = lnd_client.network().await?;
        lnd_client.network = network;

        Ok(lnd_client)
    }

    pub async fn connect(config: &LndGrpcClientConfig) -> Result<LndChannel, LightningError> {
        let endpoint_uri = config
            .endpoint
            .parse::<http::Uri>()
            .map_err(|e| LightningError::ParseConfig(format!("Invalid gRPC endpoint URI: {}", e)))?;
        let tls_domain = endpoint_uri
            .host()
            .ok_or_else(|| LightningError::ParseConfig("Missing host in gRPC endpoint URI".to_string()))?;

        let tls_cert = fs::read(PathBuf::from(&config.cert_path))
            .await
            .map_err(|e| LightningError::ReadCertificates(e.to_string()))?;
        let ca_certificate = Certificate::from_pem(tls_cert);

        let tls_config = ClientTlsConfig::new()
            .ca_certificate(ca_certificate)
            .domain_name(tls_domain);

        let channel = Channel::from_shared(config.endpoint.clone())
            .map_err(|e| LightningError::ParseConfig(e.to_string()))?
            .tls_config(tls_config)
            .map_err(|e| LightningError::ParseConfig(e.to_string()))?
            .connect()
            .await
            .map_err(|e| LightningError::Connect(Self::format_transport_error(&e)))?;

        let macaroon = read_macaroon(&config.macaroon_path)
            .await
            .map_err(|e| LightningError::ReadCertificates(e.to_string()))?;

        let macaroon_value = macaroon
            .parse::<tonic::metadata::AsciiMetadataValue>()
            .map_err(|e| LightningError::ParseConfig(format!("Invalid macaroon: {}", e)))?;

        Ok(InterceptedService::new(
            channel,
            MacaroonInterceptor {
                macaroon: macaroon_value,
            },
        ))
    }

    fn format_transport_error(err: &tonic::transport::Error) -> String {
        let mut message = err.to_string();
        let mut source = err.source();
        while let Some(next) = source {
            message.push_str(": ");
            message.push_str(&next.to_string());
            source = next.source();
        }
        message
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
            InvoiceState::Settled => {
                invoice.status = InvoiceStatus::Settled;
                invoice.payment_time = Some(Utc.timestamp_opt(response.settle_date, 0).unwrap());
                if response.amt_paid_msat > 0 {
                    invoice.amount_received_msat = Some(response.amt_paid_msat as u64);
                }
            }
            InvoiceState::Open | InvoiceState::Accepted => {
                invoice.status = InvoiceStatus::Pending;
            }
            InvoiceState::Canceled => {
                invoice.status = InvoiceStatus::Expired;
            }
        }

        Ok(invoice)
    }

    fn payment_from_lnrpc(&self, payment: lnrpc::Payment) -> Payment {
        let status = match payment.status() {
            lnrpc::payment::PaymentStatus::Succeeded => PaymentStatus::Settled,
            lnrpc::payment::PaymentStatus::Failed => PaymentStatus::Failed,
            _ => PaymentStatus::Pending,
        };

        let error = match payment.failure_reason() {
            PaymentFailureReason::FailureReasonNone => None,
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
        _label: String,
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
        let request = SendPaymentRequest {
            payment_request: bolt11,
            amt_msat: amount_msat.map(|v| v as i64).unwrap_or_default(),
            fee_limit_msat: self.fee_limit_msat as i64,
            timeout_seconds: self.payment_timeout.as_secs() as i32,
            no_inflight_updates: true,
            ..Default::default()
        };

        let mut router = self.router.clone();
        let stream = timeout(self.payment_timeout, router.send_payment_v2(request))
            .await
            .map_err(|_| LightningError::Pay("Payment timed out".to_string()))?
            .map_err(|e| LightningError::Pay(e.message().to_string()))?;

        let payment = stream
            .into_inner()
            .message()
            .await
            .map_err(|e| LightningError::Pay(e.message().to_string()))?
            .ok_or_else(|| LightningError::Pay("No payment response received".to_string()))?;

        match payment.status() {
            lnrpc::payment::PaymentStatus::Succeeded => Ok(self.payment_from_lnrpc(payment)),
            lnrpc::payment::PaymentStatus::Failed => {
                Err(LightningError::Pay(format!("{:?}", payment.failure_reason())))
            }
            status => Err(LightningError::Pay(format!("Unexpected payment status: {:?}", status))),
        }
    }

    async fn invoice_by_hash(&self, payment_hash: String) -> Result<Option<Invoice>, LightningError> {
        let hash_bytes = hex::decode(payment_hash).map_err(|e| LightningError::InvoiceByHash(e.to_string()))?;
        let request = PaymentHash {
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
        let hash_bytes = hex::decode(&payment_hash).map_err(|e| LightningError::PaymentByHash(e.to_string()))?;

        let mut router = self.router.clone();
        let result = router
            .track_payment_v2(TrackPaymentRequest {
                payment_hash: hash_bytes,
                no_inflight_updates: true,
            })
            .await;

        match result {
            Ok(response) => {
                let payment = response
                    .into_inner()
                    .message()
                    .await
                    .map_err(|e| LightningError::PaymentByHash(e.message().to_string()))?;

                Ok(payment.map(|p| self.payment_from_lnrpc(p)))
            }
            Err(err) => match err.code() {
                Code::NotFound => Ok(None),
                _ => Err(LightningError::PaymentByHash(err.message().to_string())),
            },
        }
    }

    async fn cancel_invoice(&self, payment_hash: String, _label: String) -> Result<(), LightningError> {
        let hash_bytes = hex::decode(&payment_hash).map_err(|e| LightningError::CancelInvoice(e.to_string()))?;
        let mut invoices = self.invoices.clone();
        invoices
            .cancel_invoice(CancelInvoiceMsg {
                payment_hash: hash_bytes,
            })
            .await
            .map_err(|e| LightningError::CancelInvoice(e.message().to_string()))?;

        let mut client = self.client.clone();
        client
            .delete_canceled_invoice(DelCanceledInvoiceReq {
                invoice_hash: payment_hash,
            })
            .await
            .map_err(|e| LightningError::CancelInvoice(e.message().to_string()))?;

        Ok(())
    }

    async fn health(&self) -> Result<HealthStatus, LightningError> {
        let mut client = self.client.clone();
        client
            .get_info(GetInfoRequest {})
            .await
            .map_err(|e| LightningError::HealthCheck(e.message().to_string()))?;

        Ok(HealthStatus::Operational)
    }
}

#[async_trait]
impl BitcoinWallet for LndGrpcClient {
    async fn new_address(&self, address_type: BtcAddressType) -> Result<String, BitcoinError> {
<<<<<<< ours
        let mut client = self.client.clone();

        let address_type_param = match address_type {
            BtcAddressType::P2sh => lnrpc::AddressType::NestedPubkeyHash,
            BtcAddressType::P2tr => lnrpc::AddressType::TaprootPubkey,
            BtcAddressType::P2wpkh => lnrpc::AddressType::WitnessPubkeyHash,
            _ => return Err(BitcoinError::AddressType(address_type.to_string())),
        };

        let response = client
            .new_address(NewAddressRequest {
                r#type: address_type_param as i32,
                account: String::new(),
            })
            .await
            .map_err(|e| BitcoinError::Address(e.message().to_string()))?
            .into_inner();

        Ok(response.address)
=======
        todo!("")
>>>>>>> theirs
    }

    async fn prepare_transaction(
        &self,
        address: String,
        amount_sat: u64,
        fee_rate: Option<u32>,
    ) -> Result<BtcPreparedTransaction, BitcoinError> {
<<<<<<< ours
        let mut wallet = self.wallet.clone();
        let mut outputs = std::collections::HashMap::new();
        outputs.insert(address, amount_sat);

        let target_conf = if fee_rate.is_none() { Some(1) } else { None };
        let fees = match fee_rate {
            Some(rate) => Some(fund_psbt_request::Fees::SatPerVbyte(rate as u64)),
            None => target_conf.map(fund_psbt_request::Fees::TargetConf),
        };

        let response = wallet
            .fund_psbt(walletrpc::FundPsbtRequest {
                template: Some(fund_psbt_request::Template::Raw(TxTemplate {
                    inputs: Vec::new(),
                    outputs,
                })),
                fees,
                min_confs: 1,
                spend_unconfirmed: false,
                ..Default::default()
            })
            .await
            .map_err(|e| BitcoinError::PrepareTransaction(e.message().to_string()))?
            .into_inner();

        let psbt_base64 = STANDARD.encode(&response.funded_psbt);
        let psbt = parse_psbt(&psbt_base64)?;
        let fee = psbt.fee().map_err(|e| BitcoinError::ParsePsbt(e.to_string()))?;

        let locked_utxos = response
            .locked_utxos
            .into_iter()
            .filter_map(|lease| {
                let outpoint = lease.outpoint?;
                let txid = if !outpoint.txid_str.is_empty() {
                    outpoint.txid_str
                } else if !outpoint.txid_bytes.is_empty() {
                    hex::encode(outpoint.txid_bytes)
                } else {
                    return None;
                };

                Some(BtcLockedUtxo {
                    id: hex::encode(lease.id),
                    txid,
                    output_index: outpoint.output_index,
                })
            })
            .collect();

        Ok(BtcPreparedTransaction {
            txid: psbt.unsigned_tx.compute_txid().to_string(),
            fee_sat: fee.to_sat(),
            psbt: psbt_base64,
            locked_utxos,
        })
    }

    async fn sign_send_transaction(&self, prepared: &BtcPreparedTransaction) -> Result<(), BitcoinError> {
        let mut wallet = self.wallet.clone();
        let psbt_bytes = STANDARD
            .decode(&prepared.psbt)
            .map_err(|e| BitcoinError::FinalizeTransaction(e.to_string()))?;

        let finalize_response = wallet
            .finalize_psbt(FinalizePsbtRequest {
                funded_psbt: psbt_bytes,
                account: String::new(),
            })
            .await
            .map_err(|e| BitcoinError::FinalizeTransaction(e.message().to_string()))?
            .into_inner();

        let response = wallet
            .publish_transaction(walletrpc::Transaction {
                tx_hex: finalize_response.raw_final_tx,
                label: String::new(),
            })
            .await
            .map_err(|e| BitcoinError::BroadcastTransaction(e.message().to_string()))?
            .into_inner();

        if !response.publish_error.is_empty() {
            return Err(BitcoinError::BroadcastTransaction(response.publish_error));
        }

        Ok(())
=======
        todo!("")
    }

    async fn sign_send_transaction(&self, prepared: &BtcPreparedTransaction) -> Result<(), BitcoinError> {
        todo!("")
>>>>>>> theirs
    }

    async fn release_prepared_transaction(&self, prepared: &BtcPreparedTransaction) -> Result<(), BitcoinError> {
        todo!("")
    }

    async fn get_transaction(&self, txid: &str) -> Result<Option<BtcTransaction>, BitcoinError> {
<<<<<<< ours
        let mut wallet = self.wallet.clone();
        let response = wallet
            .get_transaction(GetTransactionRequest { txid: txid.to_string() })
            .await;

        match response {
            Ok(response) => Ok(Some(Self::transaction_from_lnrpc(response.into_inner()))),
            Err(err) => match err.code() {
                Code::NotFound => Ok(None),
                _ => Err(BitcoinError::GetTransaction(err.message().to_string())),
            },
        }
=======
        todo!("")
>>>>>>> theirs
    }

    async fn synchronize(&self, cursor: Option<OnchainSyncCursor>) -> Result<OnchainSyncBatch, BitcoinError> {
        todo!("")
    }

    async fn get_output(
        &self,
        txid: &str,
        output_index: Option<u32>,
        address: Option<&str>,
        include_spent: bool,
    ) -> Result<Option<BtcOutput>, BitcoinError> {
        todo!("")
    }

    fn network(&self) -> BtcNetwork {
        self.network
    }
}
