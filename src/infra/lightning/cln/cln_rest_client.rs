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
        composition::Ledger,
        errors::{BitcoinError, LightningError},
    },
    domains::{
        bitcoin::{
            BitcoinWallet, BtcAddressType, BtcBroadcastTransaction, BtcNetwork, BtcOutput, BtcOutputStatus,
            BtcPreparedTransaction, BtcTransaction, BtcTransactionOutput, OnchainSyncBatch, OnchainSyncCursor,
            OnchainTransaction,
        },
        event::OnchainWithdrawalEvent,
        invoice::{Invoice, LnInvoiceOptions},
        payment::{LnPayment, LnPaymentTarget, Payment, PaymentStatus},
        system::HealthStatus,
    },
    infra::{
        config::config_rs::deserialize_duration,
        lightning::{bitcoin_utils::parse_psbt, cln::ListFundsResponse, types::parse_network, LnClient},
    },
};

use super::{
    DelInvoiceRequest, DelInvoiceResponse, ErrorResponse, GetRoutesRequest, GetRoutesResponse, GetinfoRequest,
    GetinfoResponse, InvoiceRequest, InvoiceResponse, ListChainMovesRequest, ListChainMovesResponse, ListFundsRequest,
    ListInvoicesRequest, ListInvoicesResponse, ListPaysRequest, ListPaysResponse, ListTransactionsRequest,
    ListTransactionsResponse, NewAddrRequest, NewAddrResponse, SetPsbtVersionRequest, SetPsbtVersionResponse,
    TxDiscardRequest, TxDiscardResponse, TxPrepareOutput, TxPrepareRequest, TxPrepareResponse, TxSendRequest,
    TxSendResponse, XpayRequest, XpayResponse,
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
    #[serde(deserialize_with = "deserialize_duration")]
    pub payment_timeout: Duration,
    /// Absolute max routing fee per payment, in msat. When unset, `xpay` applies
    /// its own default of `max(1%, 5000 msat)`.
    pub maxfee: Option<u64>,
    #[serde(deserialize_with = "deserialize_duration")]
    pub ws_min_reconnect_delay: Duration,
    #[serde(deserialize_with = "deserialize_duration")]
    pub ws_max_reconnect_delay: Duration,
    pub ca_cert_path: Option<String>,
}

pub struct ClnRestClient {
    client: Client,
    base_url: String,
    node_id: String,
    maxfee: Option<u64>,
    retry_for: Option<u32>,
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
            // Rustls requires an explicit trust store when hostname verification is disabled.
            client_builder = if config.accept_invalid_hostnames {
                client_builder.tls_certs_only([ca_certificate])
            } else {
                client_builder.tls_certs_merge([ca_certificate])
            };
            client_builder = client_builder.tls_danger_accept_invalid_hostnames(config.accept_invalid_hostnames);
        }

        let client = client_builder
            .build()
            .map_err(|e| LightningError::ParseConfig(format!("{e:?}")))?;

        let mut cln_client = Self {
            client,
            base_url: config.endpoint.clone(),
            node_id: String::new(),
            maxfee: config.maxfee,
            retry_for: Some(config.payment_timeout.as_secs() as u32),
            network: BtcNetwork::default(),
        };

        let node = cln_client.node_info().await?;
        cln_client.network = parse_network(&node.network);
        cln_client.node_id = node.id;

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

    async fn node_info(&self) -> Result<GetinfoResponse, LightningError> {
        self.post_request("getinfo", &GetinfoRequest {})
            .await
            .map_err(|e| LightningError::NodeInfo(e.to_string()))
    }
}

#[async_trait]
impl LnClient for ClnRestClient {
    fn offer_backend(&self) -> Option<String> {
        Some(self.withdrawal_backend())
    }
    async fn issue_offer(
        &self,
        offer: &swissknife_types::WalletOffer,
    ) -> Result<crate::domains::offer::NodeOffer, LightningError> {
        let label = offer.id.to_string();
        let request = serde_json::json!({"amount":offer.amount_msat.map_or_else(||"any".into(),|a|format!("{a}msat")),"description":offer.description,"label":label,"issuer":format!("SwissKnife {label}"),"single_use":offer.single_use});
        self.post_request("offer", &request)
            .await
            .map_err(|e| LightningError::Invoice(e.to_string()))
    }
    async fn local_offers(&self, id: Option<String>) -> Result<Vec<crate::domains::offer::NodeOffer>, LightningError> {
        #[derive(serde::Deserialize)]
        struct Offers {
            offers: Vec<crate::domains::offer::NodeOffer>,
        }
        let request = id.map_or_else(|| serde_json::json!({}), |id| serde_json::json!({"offer_id":id}));
        self.post_request::<Offers>("listoffers", &request)
            .await
            .map(|r| r.offers)
            .map_err(|e| LightningError::Invoice(e.to_string()))
    }
    async fn disable_offer(&self, id: &str) -> Result<(), LightningError> {
        self.post_request::<crate::domains::offer::NodeOffer>("disableoffer", &serde_json::json!({"offer_id":id}))
            .await
            .map(|_| ())
            .map_err(|e| LightningError::Invoice(e.to_string()))
    }
    async fn offer_updates(&self, start: u64) -> Result<Vec<crate::domains::offer::OfferUpdate>, LightningError> {
        #[derive(serde::Deserialize)]
        struct Updates {
            invoices: Vec<crate::domains::offer::OfferUpdate>,
        }
        self.post_request::<Updates>(
            "listinvoices",
            &serde_json::json!({"index":"updated","start":start,"limit":100}),
        )
        .await
        .map(|r| r.invoices)
        .map_err(|e| LightningError::Invoice(e.to_string()))
    }
    async fn decode_bolt12(&self, input: &str) -> Result<crate::domains::payment::DecodedBolt12, LightningError> {
        self.post_request("decode", &serde_json::json!({"string":input}))
            .await
            .map_err(|e| LightningError::InvalidPaymentTarget(e.to_string()))
    }

    async fn bip353_instructions(
        &self,
        name: &str,
    ) -> Result<crate::domains::payment::Bip353Instructions, LightningError> {
        self.post_request("fetchbip353", &serde_json::json!({"address":name}))
            .await
            .map_err(|e| LightningError::InvalidPaymentTarget(e.to_string()))
    }
    async fn fetch_offer_invoice(
        &self,
        offer: &str,
        amount_msat: u64,
        note: Option<String>,
        bip353: Option<String>,
    ) -> Result<String, LightningError> {
        #[derive(serde::Deserialize)]
        struct FetchedInvoice {
            invoice: String,
        }
        let mut request = serde_json::json!({"offer":offer,"amount_msat":amount_msat,"timeout":15});
        if let Some(name) = bip353 {
            request["bip353"] = name.into();
        }
        if let Some(note) = note {
            request["payer_note"] = serde_json::Value::String(note.to_owned());
        }
        self.post_request::<FetchedInvoice>("fetchinvoice", &request)
            .await
            .map(|r| r.invoice)
            .map_err(|e| LightningError::InvalidPaymentTarget(e.to_string()))
    }

    async fn disconnect(&self) -> Result<(), LightningError> {
        Ok(())
    }

    async fn invoice(
        &self,
        amount_msat: u64,
        description: String,
        label: String,
        expiry: u32,
        options: LnInvoiceOptions,
    ) -> Result<Invoice, LightningError> {
        let response: InvoiceResponse = self
            .post_request(
                "invoice",
                &InvoiceRequest {
                    description,
                    expiry: expiry as u64,
                    label,
                    amount_msat,
                    deschashonly: Some(options.description_hash_only),
                    preimage: options.preimage.map(hex::encode),
                },
            )
            .await
            .map_err(|e| LightningError::Invoice(e.to_string()))?;

        let bolt11 = Bolt11Invoice::from_str(&response.bolt11).map_err(|e| LightningError::Invoice(e.to_string()))?;

        Ok(crate::infra::lightning::types::invoice_from_bolt11(bolt11))
    }

    async fn estimate_fee(&self, target: LnPaymentTarget) -> Result<u64, LightningError> {
        let fee_limit_msat = self.fee_limit_msat(target.amount_msat);
        let response: GetRoutesResponse = self
            .post_request(
                "getroutes",
                &GetRoutesRequest {
                    source: self.node_id.clone(),
                    destination: hex::encode(target.destination),
                    amount_msat: target.amount_msat,
                    layers: vec!["auto.localchans".to_string(), "auto.sourcefree".to_string()],
                    maxfee_msat: fee_limit_msat,
                    final_cltv: target.final_cltv_delta,
                },
            )
            .await
            .map_err(|err| LightningError::EstimateFee(err.to_string()))?;
        if response.routes.is_empty() {
            return Err(LightningError::EstimateFee("No route available".to_string()));
        }

        let estimated_fee_msat = response.routes.iter().try_fold(0_u64, |total, route| {
            let sent =
                route.path.first().and_then(|hop| hop.amount_in_msat).ok_or_else(|| {
                    LightningError::EstimateFee("Estimated route has no first hop amount".to_string())
                })?;

            Ok(total.saturating_add(sent.saturating_sub(route.amount_msat)))
        })?;

        Ok(estimated_fee_msat.min(fee_limit_msat))
    }

    fn fee_limit_msat(&self, amount_msat: u64) -> u64 {
        self.maxfee.unwrap_or_else(|| amount_msat.div_ceil(100).max(5_000))
    }

    async fn pay(
        &self,
        bolt11: String,
        amount_msat: Option<u64>,
        fee_limit_msat: u64,
        label: String,
    ) -> Result<Payment, LightningError> {
        let response: XpayResponse = self
            .post_request(
                "xpay",
                &XpayRequest {
                    invstring: bolt11,
                    amount_msat,
                    maxfee: Some(fee_limit_msat),
                    retry_for: self.retry_for,
                    label: Some(label),
                },
            )
            .await
            .map_err(|e| LightningError::PaymentOutcomeUnknown(e.to_string()))?;

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
            Some(invoice) => invoice.bolt11_invoice(),
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

        let payment = response.pays.into_iter().rfind(|pay| pay.payment_hash == payment_hash);

        let Some(payment) = payment else {
            return Ok(None);
        };

        let amount_msat = payment.amount_msat.unwrap_or_default();
        let amount_sent_msat = payment.amount_sent_msat.unwrap_or(amount_msat);
        let payment_time = payment
            .completed_at
            .or(payment.created_at)
            .and_then(|timestamp| Utc.timestamp_opt(timestamp as i64, 0).single());

        let status = match payment.status.as_str() {
            "complete" => PaymentStatus::Settled,
            "failed" => PaymentStatus::Failed,
            "pending" => PaymentStatus::Pending,
            status => {
                return Err(LightningError::PaymentByHash(format!(
                    "Unknown payment status: {status}"
                )))
            }
        };

        Ok(Some(Payment {
            ledger: Ledger::Lightning,
            status,
            amount_msat,
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

    fn withdrawal_backend(&self) -> String {
        format!("cln:{}:{}", self.network, self.node_id)
    }

    async fn plan_withdrawal(
        &self,
        address: String,
        amount_sat: u64,
        _lock_id: [u8; 32],
    ) -> Result<BtcPreparedTransaction, BitcoinError> {
        let (expected, startweight, msat) = super::withdrawal::destination(&address, amount_sat)?;
        let funded:SetPsbtVersionResponse = self.post_request("fundpsbt",&serde_json::json!({"satoshi":format!("{msat}msat"),"feerate":"normal","startweight":startweight,"minconf":1,"reserve":0,"excess_as_change":true,"nonwrapped":true})).await.map_err(|e|BitcoinError::PrepareTransaction(e.to_string()))?;
        let output: super::cln_rest_types::AddPsbtOutputResponse = self
            .post_request(
                "addpsbtoutput",
                &serde_json::json!({"satoshi":format!("{msat}msat"),"initialpsbt":funded.psbt,"destination":address}),
            )
            .await
            .map_err(|e| BitcoinError::PrepareTransaction(e.to_string()))?;
        let v0: SetPsbtVersionResponse = self
            .post_request(
                "setpsbtversion",
                &SetPsbtVersionRequest {
                    psbt: output.psbt.clone(),
                    version: 0,
                },
            )
            .await
            .map_err(|e| BitcoinError::ParsePsbt(e.to_string()))?;
        super::withdrawal::plan(&v0.psbt, output.psbt, output.outnum, expected)
    }

    async fn sign_withdrawal(
        &self,
        plan: &BtcPreparedTransaction,
        _lock_id: [u8; 32],
    ) -> Result<BtcBroadcastTransaction, BitcoinError> {
        self.post_request::<serde_json::Value>(
            "reserveinputs",
            &serde_json::json!({"psbt":plan.psbt,"exclusive":true,"reserve":super::withdrawal::RESERVATION_BLOCKS}),
        )
        .await
        .map_err(|e| BitcoinError::FinalizeTransaction(e.to_string()))?;
        let signed: super::cln_rest_types::SignedPsbtResponse = self
            .post_request("signpsbt", &serde_json::json!({"psbt":plan.psbt}))
            .await
            .map_err(|e| BitcoinError::FinalizeTransaction(e.to_string()))?;
        let signed_v0: SetPsbtVersionResponse = self
            .post_request(
                "setpsbtversion",
                &serde_json::json!({"psbt":signed.signed_psbt, "version":0}),
            )
            .await
            .map_err(|e| BitcoinError::FinalizeTransaction(e.to_string()))?;
        if parse_psbt(&signed_v0.psbt)?.unsigned_tx.compute_txid().to_string() != plan.txid {
            return Err(BitcoinError::FinalizeTransaction(
                "Signed PSBT differs from the approved withdrawal".into(),
            ));
        }
        Ok(BtcBroadcastTransaction::SignedPsbt {
            txid: plan.txid.clone(),
            psbt: signed.signed_psbt,
        })
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

    async fn broadcast_transaction(&self, transaction: &BtcBroadcastTransaction) -> Result<(), BitcoinError> {
        if let BtcBroadcastTransaction::SignedPsbt { txid, psbt } = transaction {
            let response: TxSendResponse = self
                .post_request(
                    "sendpsbt",
                    &serde_json::json!({"psbt":psbt,"reserve":super::withdrawal::RESERVATION_BLOCKS}),
                )
                .await
                .map_err(|e| BitcoinError::BroadcastTransaction(e.to_string()))?;
            let actual = response.txid;
            return if actual == *txid {
                Ok(())
            } else {
                Err(BitcoinError::BroadcastTransaction(
                    "Published transaction ID differs from the signed plan".into(),
                ))
            };
        }
        let BtcBroadcastTransaction::Prepared(txid) = transaction else {
            return Err(BitcoinError::BroadcastTransaction(
                "Core Lightning requires a prepared transaction ID".into(),
            ));
        };
        self.post_request::<TxSendResponse>("txsend", &TxSendRequest { txid: txid.clone() })
            .await
            .map_err(|e| BitcoinError::BroadcastTransaction(e.to_string()))?;

        Ok(())
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
            unsigned_txid: None,
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
                        unsigned_txid: None,
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

    async fn get_output<'a>(
        &self,
        txid: &str,
        output_index: Option<u32>,
        address: Option<&'a str>,
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
