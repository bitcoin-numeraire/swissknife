use std::{collections::HashMap, path::PathBuf, str::FromStr, time::Duration};

use async_trait::async_trait;
use bitcoin::{Address, Network, OutPoint, ScriptBuf};
use chrono::{TimeZone, Utc};
use lightning_invoice::Bolt11Invoice;
use reqwest::{
    header::{HeaderMap, HeaderValue},
    Certificate, Client,
};
use serde::{de::DeserializeOwned, Deserialize, Serialize};
use tokio::fs;

use crate::{
    application::{
        entities::Ledger,
        errors::{BitcoinError, LightningError},
    },
    domains::{
        bitcoin::{
            BitcoinWallet, BtcAddressType, BtcNetwork, BtcOutput, BtcOutputStatus, BtcPreparedTransaction,
            BtcTransaction, BtcTransactionOutput, OnchainSyncBatch, OnchainSyncCursor, OnchainTransaction,
        },
        event::OnchainWithdrawalEvent,
        invoice::Invoice,
        payment::{LnPayment, Payment, PaymentStatus},
        system::HealthStatus,
    },
    infra::{
        config::config_rs::deserialize_duration,
        lightning::{bitcoin_utils::parse_psbt, cln::ListFundsResponse, types::parse_network, LnClient},
    },
};

use super::{
    DelInvoiceRequest, DelInvoiceResponse, ErrorResponse, GetinfoRequest, GetinfoResponse, InvoiceRequest,
    InvoiceResponse, ListChainMovesRequest, ListChainMovesResponse, ListFundsRequest, ListInvoicesRequest,
    ListInvoicesResponse, ListPaysRequest, ListPaysResponse, ListTransactionsRequest, ListTransactionsResponse,
    NewAddrRequest, NewAddrResponse, PayRequest, PayResponse, SetPsbtVersionRequest, SetPsbtVersionResponse,
    TxDiscardRequest, TxDiscardResponse, TxPrepareOutput, TxPrepareRequest, TxPrepareResponse, TxSendRequest,
    TxSendResponse,
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
        label: String,
        expiry: u32,
        deschashonly: bool,
    ) -> Result<Invoice, LightningError> {
        let response: InvoiceResponse = self
            .post_request(
                "invoice",
                &InvoiceRequest {
                    description,
                    expiry: expiry as u64,
                    label,
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
        let response = self
            .post_request::<ListPaysResponse>(
                "listpays",
                &ListPaysRequest {
                    payment_hash: Some(payment_hash.clone()),
                },
            )
            .await
            .map_err(|e| LightningError::PaymentByHash(e.to_string()))?;

        let payment = response
            .pays
            .into_iter()
            .find(|pay| matches!(pay.status.as_str(), "complete" | "failed") && pay.payment_hash == payment_hash);

        let Some(payment) = payment else {
            return Ok(None);
        };

        let amount_msat = payment.amount_msat.unwrap_or_default();
        let amount_sent_msat = payment.amount_sent_msat.unwrap_or(amount_msat);
        let payment_time = payment
            .completed_at
            .or(payment.created_at)
            .and_then(|timestamp| Utc.timestamp_opt(timestamp as i64, 0).single());

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
                payment_hash,
                payment_preimage: payment.preimage,
                ..Default::default()
            }),
            ..Default::default()
        }))
    }

    async fn cancel_invoice(&self, _payment_hash: String, label: String) -> Result<(), LightningError> {
        self.post_request::<DelInvoiceResponse>(
            "delinvoice",
            &DelInvoiceRequest {
                label,
                status: "unpaid".to_string(),
                desconly: None,
            },
        )
        .await
        .map_err(|e| LightningError::CancelInvoice(e.to_string()))?;

        Ok(())
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
        let address_type_param = match address_type {
            BtcAddressType::P2wpkh => "bech32",
            BtcAddressType::P2tr => "p2tr",
            _ => return Err(BitcoinError::AddressType(address_type.to_string())),
        };

        let response: NewAddrResponse = self
            .post_request(
                "newaddr",
                &NewAddrRequest {
                    addresstype: Some(address_type_param.to_string()),
                },
            )
            .await
            .map_err(|e| BitcoinError::Address(e.to_string()))?;

        response
            .bech32
            .or(response.p2tr)
            .ok_or_else(|| BitcoinError::Address("No address returned by CLN".to_string()))
    }

    async fn prepare_transaction(
        &self,
        address: String,
        amount_sat: u64,
        fee_rate_sat_vb: Option<u32>,
    ) -> Result<BtcPreparedTransaction, BitcoinError> {
        let response: TxPrepareResponse = self
            .post_request(
                "txprepare",
                &TxPrepareRequest {
                    outputs: vec![TxPrepareOutput {
                        address,
                        amount: amount_sat,
                    }],
                    feerate: fee_rate_sat_vb.map(|rate| rate * 1000), // Convert sat/vbyte to perkb
                },
            )
            .await
            .map_err(|e| BitcoinError::PrepareTransaction(e.to_string()))?;

        let set_psbt_version_response: SetPsbtVersionResponse = self
            .post_request(
                "setpsbtversion",
                &SetPsbtVersionRequest {
                    psbt: response.psbt.clone(),
                    version: 0,
                },
            )
            .await
            .map_err(|e| BitcoinError::PrepareTransaction(e.to_string()))?;

        let psbt = parse_psbt(&set_psbt_version_response.psbt)?;
        let fee = psbt.fee().map_err(|e| BitcoinError::ParsePsbt(e.to_string()))?;

        Ok(BtcPreparedTransaction {
            txid: response.txid,
            fee_sat: fee.to_sat(),
            psbt: response.psbt,
            locked_utxos: Vec::new(),
        })
    }

    async fn sign_send_transaction(&self, prepared: &BtcPreparedTransaction) -> Result<Option<String>, BitcoinError> {
        self.post_request::<TxSendResponse>(
            "txsend",
            &TxSendRequest {
                txid: prepared.txid.clone(),
            },
        )
        .await
        .map_err(|e| BitcoinError::FinalizeTransaction(e.to_string()))?;

        Ok(None)
    }

    async fn release_prepared_transaction(&self, prepared: &BtcPreparedTransaction) -> Result<(), BitcoinError> {
        let _response: TxDiscardResponse = self
            .post_request(
                "txdiscard",
                &TxDiscardRequest {
                    txid: prepared.txid.clone(),
                },
            )
            .await
            .map_err(|e| BitcoinError::ReleaseTransaction(e.to_string()))?;

        Ok(())
    }

    async fn get_transaction(&self, txid: &str) -> Result<Option<BtcTransaction>, BitcoinError> {
        let response: ListTransactionsResponse = self
            .post_request("listtransactions", &ListTransactionsRequest {})
            .await
            .map_err(|e| BitcoinError::GetTransaction(e.to_string()))?;

        let Some(transaction) = response
            .transactions
            .into_iter()
            .find(|transaction| transaction.hash == txid)
        else {
            return Ok(None);
        };

        let network = match self.network {
            BtcNetwork::Bitcoin => Network::Bitcoin,
            BtcNetwork::Testnet | BtcNetwork::Testnet4 => Network::Testnet,
            BtcNetwork::Regtest => Network::Regtest,
            BtcNetwork::Signet => Network::Signet,
            BtcNetwork::Simnet => Network::Regtest,
        };

        let outputs: Result<Vec<_>, BitcoinError> = transaction
            .outputs
            .into_iter()
            .map(|output| {
                let script_bytes =
                    hex::decode(&output.script_pub_key).map_err(|e| BitcoinError::GetTransaction(e.to_string()))?;
                let script = ScriptBuf::from_bytes(script_bytes);
                let address =
                    Address::from_script(&script, network).map_err(|e| BitcoinError::GetTransaction(e.to_string()))?;

                Ok(BtcTransactionOutput {
                    output_index: output.index,
                    address: address.to_string(),
                    amount_sat: output.amount_msat / 1000,
                    is_ours: false,
                })
            })
            .collect();
        let outputs = outputs?;

        Ok(Some(BtcTransaction {
            txid: transaction.hash,
            block_height: Some(transaction.blockheight),
            outputs,
            is_outgoing: false, // TODO: No way for now to determine this from CLN
        }))
    }

    async fn synchronize(&self, cursor: Option<OnchainSyncCursor>) -> Result<OnchainSyncBatch, BitcoinError> {
        let start_index = match cursor {
            Some(OnchainSyncCursor::CreatedIndex(index)) => index,
            _ => 0,
        };

        let response: ListChainMovesResponse = self
            .post_request(
                "listchainmoves",
                &ListChainMovesRequest {
                    index: Some("created".to_string()),
                    start: Some(start_index),
                    limit: None,
                },
            )
            .await
            .map_err(|e| BitcoinError::Synchronize(e.to_string()))?;

        if response.chainmoves.is_empty() {
            return Ok(OnchainSyncBatch {
                events: Vec::new(),
                next_cursor: None,
            });
        }

        let has_deposits = response
            .chainmoves
            .iter()
            .any(|m| m.primary_tag == "deposit" && m.account_id == "wallet");

        let outputs_by_outpoint = if has_deposits {
            let funds: ListFundsResponse = self
                .post_request("listfunds", &ListFundsRequest { spent: Some(true) })
                .await
                .map_err(|e| BitcoinError::Synchronize(e.to_string()))?;

            let mut map = HashMap::new();
            for output in funds.outputs {
                let outpoint = format!("{}:{}", output.txid, output.output);
                let status = match output.status.as_str() {
                    "confirmed" => BtcOutputStatus::Confirmed,
                    "spent" | "unconfirmed" => BtcOutputStatus::Unconfirmed,
                    _ => BtcOutputStatus::Unconfirmed,
                };

                map.insert(
                    outpoint,
                    BtcOutput {
                        txid: output.txid.clone(),
                        output_index: output.output,
                        address: output.address.unwrap_or_default(),
                        amount_sat: output.amount_msat / 1000,
                        block_height: output.blockheight,
                        outpoint: format!("{}:{}", output.txid, output.output),
                        status,
                        ..Default::default()
                    },
                );
            }
            map
        } else {
            HashMap::new()
        };

        let mut max_index = start_index;
        let mut events = Vec::new();

        for chainmove in response.chainmoves {
            max_index = max_index.max(chainmove.created_index);

            match (chainmove.primary_tag.as_str(), chainmove.account_id.as_str()) {
                ("deposit", "wallet") => {
                    let outpoint =
                        OutPoint::from_str(&chainmove.utxo).map_err(|e| BitcoinError::Synchronize(e.to_string()))?;
                    let key = format!("{}:{}", outpoint.txid, outpoint.vout);

                    if let Some(output) = outputs_by_outpoint.get(&key) {
                        events.push(OnchainTransaction::Deposit(output.clone()));
                    }
                }
                ("withdrawal", "wallet") => {
                    let Some(spending_txid) = chainmove.spending_txid.clone() else {
                        continue;
                    };

                    events.push(OnchainTransaction::Withdrawal(OnchainWithdrawalEvent {
                        txid: spending_txid,
                        block_height: chainmove.blockheight,
                    }));
                }
                _ => {}
            }
        }

        let next_cursor = if max_index > start_index {
            Some(OnchainSyncCursor::CreatedIndex(max_index.saturating_add(1)))
        } else {
            None
        };

        Ok(OnchainSyncBatch { events, next_cursor })
    }

    async fn get_output(
        &self,
        txid: &str,
        output_index: Option<u32>,
        address: Option<&str>,
        include_spent: bool,
    ) -> Result<Option<BtcOutput>, BitcoinError> {
        let response: ListFundsResponse = self
            .post_request(
                "listfunds",
                &ListFundsRequest {
                    spent: Some(include_spent),
                },
            )
            .await
            .map_err(|e| BitcoinError::GetOutput(e.to_string()))?;

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

        Ok(output.map(|output| {
            let status = match output.status.as_str() {
                "unconfirmed" => BtcOutputStatus::Unconfirmed,
                "confirmed" => BtcOutputStatus::Confirmed,
                "spent" => BtcOutputStatus::Spent,
                "immature" => BtcOutputStatus::Immature,
                _ => BtcOutputStatus::default(),
            };

            BtcOutput {
                txid: output.txid.clone(),
                output_index: output.output,
                address: output.address.unwrap_or_default(),
                amount_sat: output.amount_msat / 1000,
                block_height: output.blockheight,
                outpoint: format!("{}:{}", output.txid, output.output),
                status,
                ..Default::default()
            }
        }))
    }

    fn network(&self) -> BtcNetwork {
        self.network
    }
}
