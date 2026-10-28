//! SwissKnife's wallet mapping over the independently reusable public RPC client.
use std::{str::FromStr, time::Duration};

use async_trait::async_trait;
use bitcoin::hashes::{sha256, Hash};
use chrono::{DateTime, Utc};
use lightning_invoice::Bolt11Invoice;
use serde::Deserialize;
use tokio::{fs, time::Instant};
use wavelength_client::{exact_sats, rpc, swaps, Client, Config, Error};

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
pub struct WavelengthConfig {
    pub endpoint: String,
    pub cert_path: String,
    pub macaroon_path: String,
    pub fee_limit_msat: u64,
    /// Explicit consent to receive/spend operator credit. Credit top-ups from
    /// Ark funds remain disabled; they need their own authorization contract.
    #[serde(default)]
    pub allow_server_credit: bool,
    #[serde(deserialize_with = "deserialize_duration")]
    pub request_timeout: Duration,
    #[serde(deserialize_with = "deserialize_duration")]
    pub payment_timeout: Duration,
    #[serde(deserialize_with = "deserialize_duration")]
    pub poll_interval: Duration,
    pub activity_page_size: u32,
    pub activity_max_pages: usize,
}

struct QuoteValues<'a> {
    hash: &'a str,
    principal: u64,
    fee: u64,
    ark_outflow: u64,
    expires: i64,
    credit_applied: u64,
    credit_topup: u64,
    credit_shortfall: u64,
}

pub struct WavelengthClient {
    client: Client,
    config: WavelengthConfig,
    network: BtcNetwork,
}

impl WavelengthClient {
    pub async fn new(config: WavelengthConfig) -> Result<Self, LightningError> {
        if config.request_timeout.is_zero()
            || config.payment_timeout.is_zero()
            || config.poll_interval.is_zero()
            || config.activity_page_size == 0
            || config.activity_max_pages == 0
        {
            return Err(LightningError::ParseConfig(
                "Wavelength timeouts and activity bounds must be positive".into(),
            ));
        }
        let (certificate_pem, macaroon) =
            tokio::try_join!(fs::read(&config.cert_path), fs::read(&config.macaroon_path))
                .map_err(|e| LightningError::ReadCertificates(e.to_string()))?;
        let client = Client::connect(Config {
            endpoint: config.endpoint.clone(),
            certificate_pem,
            macaroon,
            connect_timeout: config.request_timeout,
            request_timeout: config.request_timeout,
        })
        .await
        .map_err(|e| LightningError::Connect(e.to_string()))?;
        let status = client
            .status(Default::default())
            .await
            .map_err(|e| LightningError::NodeInfo(e.to_string()))?;
        if !status.ready {
            return Err(LightningError::Connect(
                "waved wallet is not ready; initialize and unlock the external daemon".into(),
            ));
        }
        let network = match status.network.as_str() {
            "mainnet" | "bitcoin" => BtcNetwork::Bitcoin,
            "testnet" | "testnet3" => BtcNetwork::Testnet,
            "testnet4" => BtcNetwork::Testnet4,
            "signet" => BtcNetwork::Signet,
            "regtest" => BtcNetwork::Regtest,
            "simnet" => BtcNetwork::Simnet,
            _ => return Err(LightningError::NodeInfo("waved returned an unknown network".into())),
        };
        Ok(Self {
            client,
            config,
            network,
        })
    }

    pub fn network(&self) -> BtcNetwork {
        self.network
    }

    fn amount_sat(amount_msat: u64) -> Result<u64, LightningError> {
        exact_sats(amount_msat)
            .filter(|amount| *amount > 0 && *amount <= i64::MAX as u64)
            .ok_or(LightningError::Unsupported(
                "Wavelength requires a positive whole-satoshi amount",
            ))
    }

    fn fee_cap_sat(&self, requested: u64) -> Result<u64, LightningError> {
        // Rounding DOWN preserves the caller's authorization. In waved, zero
        // means uncapped, so a zero/sub-sat budget must never cross that boundary.
        let cap = requested.min(self.config.fee_limit_msat) / 1000;
        if cap == 0 {
            return Err(LightningError::Unsupported(
                "waved cannot enforce a fee budget below one satoshi",
            ));
        }
        Ok(cap)
    }

    async fn find(&self, hash: &str, kind: rpc::EntryKind) -> Result<Option<rpc::WalletEntry>, LightningError> {
        let activity = self
            .client
            .activity(self.config.activity_page_size, self.config.activity_max_pages)
            .await
            .map_err(|e| LightningError::PaymentByHash(e.to_string()))?;
        let mut found = None;
        for entry in activity.into_iter().filter(|entry| entry.kind == kind as i32) {
            let request = lightning_request(&entry);
            let progress_hash = entry
                .progress
                .as_ref()
                .map(|p| p.payment_hash.as_str())
                .unwrap_or_default();
            let request_hash = request.map(|r| r.payment_hash.as_str()).unwrap_or_default();
            let invoice_hash = request
                .filter(|r| !r.invoice.is_empty())
                .map(|r| Bolt11Invoice::from_str(&r.invoice).map(|invoice| invoice.payment_hash().to_string()))
                .transpose()
                .map_err(|_| invalid("activity contains an invalid invoice"))?;
            if progress_hash == hash || request_hash == hash || invoice_hash.as_deref() == Some(hash) {
                if found.is_some() {
                    return Err(invalid("multiple activity rows match one payment hash and direction"));
                }
                validate_invoice(&entry, hash)?;
                found = Some(entry);
            }
        }
        Ok(found)
    }

    async fn payment_from_swap(&self, summary: swaps::SwapSummary, hash: &str) -> Result<Payment, LightningError> {
        let invoice =
            Bolt11Invoice::from_str(&summary.invoice).map_err(|_| invalid("persisted swap invoice is invalid"))?;
        if summary.payment_hash != hash
            || summary.direction != swaps::SwapDirection::Pay as i32
            || invoice.payment_hash().to_string() != hash
        {
            return Err(invalid("persisted swap hash or direction does not match the payment"));
        }
        let amount_msat = invoice
            .amount_milli_satoshis()
            .ok_or_else(|| invalid("amountless send history"))?;
        Self::amount_sat(amount_msat)?;
        let fee_msat = summary
            .fee_sat
            .checked_mul(1000)
            .ok_or_else(|| invalid("swap fee overflow"))?;
        let state = swaps::SwapState::try_from(summary.state).map_err(|_| invalid("unknown swap state"))?;
        let mut status = PaymentStatus::Pending;
        if summary.settlement_type == swaps::SwapSettlementType::Credit as i32 {
            status = self.credit_payment_status(hash).await?;
        } else if !summary.pending {
            match state {
                swaps::SwapState::Completed => status = PaymentStatus::Settled,
                swaps::SwapState::Refunded => status = PaymentStatus::Failed,
                swaps::SwapState::Expired | swaps::SwapState::Failed
                    if summary.vhtlc_outpoint.is_empty()
                        && summary.funding_session_id.is_empty()
                        && summary.preimage.is_empty() =>
                {
                    status = PaymentStatus::Failed
                }
                _ => {}
            }
        }
        let preimage = (!summary.preimage.is_empty()).then_some(summary.preimage);
        if status == PaymentStatus::Settled {
            let proof = preimage.as_ref().and_then(|s| hex::decode(s).ok());
            if !proof
                .as_ref()
                .is_some_and(|p| p.len() == 32 && sha256::Hash::hash(p).to_string() == hash)
            {
                return Err(invalid("completed payment lacks a matching settlement proof"));
            }
        }
        Ok(Payment {
            amount_msat,
            fee_msat: Some(fee_msat),
            status,
            payment_time: Some(timestamp(summary.updated_at_unix)?),
            ledger: Ledger::Lightning,
            lightning: Some(LnPayment {
                payment_hash: hash.to_owned(),
                payment_preimage: preimage,
                ..Default::default()
            }),
            error: (!summary.terminal_reason.is_empty()).then_some(summary.terminal_reason),
            ..Default::default()
        })
    }

    async fn credit_payment_status(&self, hash: &str) -> Result<PaymentStatus, LightningError> {
        // A local credit-FSM poll cap can mark the friendly row FAILED while
        // the server payment is still running. Its bounded snapshot needs
        // positive terminal evidence; absence never proves failure.
        let snapshot = self
            .client
            .list_credits(swaps::ListCreditsRequest { limit: 1000 })
            .await;
        if let Ok(snapshot) = &snapshot {
            let mut operations = snapshot
                .operations
                .iter()
                .filter(|op| op.payment_hash == hash && op.r#type == swaps::CreditOperationType::Pay as i32);
            let operation = operations.next();
            if operations.next().is_some() {
                return Err(invalid("multiple credit pay operations for one hash"));
            }
            if let Some(operation) = operation {
                return Ok(match swaps::CreditOperationState::try_from(operation.state).ok() {
                    Some(swaps::CreditOperationState::Debited) => PaymentStatus::Settled,
                    Some(
                        swaps::CreditOperationState::Released
                        | swaps::CreditOperationState::Failed
                        | swaps::CreditOperationState::Expired,
                    ) => PaymentStatus::Failed,
                    _ => PaymentStatus::Pending,
                });
            }
        }
        // A persisted completed credit activity row records that the credit
        // FSM previously observed DEBITED. It remains usable after the server
        // operation ages out of the unpaginated credit snapshot.
        if self
            .find(hash, rpc::EntryKind::Send)
            .await?
            .is_some_and(|entry| entry.status == rpc::EntryStatus::Complete as i32)
        {
            return Ok(PaymentStatus::Settled);
        }
        snapshot.map_err(|e| LightningError::PaymentByHash(e.to_string()))?;
        Ok(PaymentStatus::Pending)
    }

    fn validate_quote(
        &self,
        quote: &rpc::PrepareSendResponse,
        hash: &str,
        principal: u64,
        cap: u64,
    ) -> Result<(), LightningError> {
        if quote.send_intent_id.is_empty()
            || !quote.fee_known
            || !quote.total_outflow_known
            || quote.quote_status != rpc::SendQuoteStatus::Complete as i32
        {
            return Err(LightningError::InvalidQuote(
                "prepared send has no complete quote or intent".into(),
            ));
        }
        let credit = quote.credit_preview.as_ref();
        let values = QuoteValues {
            hash: &quote.payment_hash,
            principal: u64::try_from(quote.amount_sat)
                .map_err(|_| LightningError::InvalidQuote("negative principal".into()))?,
            fee: u64::try_from(quote.expected_fee_sat)
                .map_err(|_| LightningError::InvalidQuote("negative fee".into()))?,
            ark_outflow: u64::try_from(quote.expected_total_outflow_sat)
                .map_err(|_| LightningError::InvalidQuote("negative outflow".into()))?,
            expires: quote.expires_at_unix,
            credit_applied: credit.map_or(0, |c| c.credit_applied_sat),
            credit_topup: credit.map_or(0, |c| c.credit_topup_sat),
            credit_shortfall: credit.map_or(0, |c| c.credit_shortfall_sat),
        };
        self.validate_quote_values(values, hash, principal, cap).map(|_| ())
    }

    fn validate_quote_values(
        &self,
        quote: QuoteValues<'_>,
        hash: &str,
        principal: u64,
        cap: u64,
    ) -> Result<u64, LightningError> {
        if quote.credit_topup > 0
            || quote.credit_shortfall > 0
            || (!self.config.allow_server_credit && quote.credit_applied > 0)
        {
            return Err(LightningError::InvalidQuote(
                "quote requires unauthorized operator credit or credit top-up".into(),
            ));
        }
        let total = quote.ark_outflow.checked_add(quote.credit_applied);
        if quote.hash != hash
            || quote.principal != principal
            || quote.fee > cap
            || total != principal.checked_add(quote.fee)
            || total.is_none()
            || quote.expires <= Utc::now().timestamp()
        {
            return Err(LightningError::InvalidQuote(
                "quote is inconsistent, expired or exceeds its authorized fee budget".into(),
            ));
        }
        quote
            .fee
            .checked_mul(1000)
            .ok_or_else(|| LightningError::InvalidQuote("fee overflow".into()))
    }
}

fn invalid(reason: &str) -> LightningError {
    LightningError::PaymentByHash(format!("Invalid Wavelength response: {reason}"))
}

fn timestamp(seconds: i64) -> Result<DateTime<Utc>, LightningError> {
    DateTime::from_timestamp(seconds, 0)
        .filter(|_| seconds > 0)
        .ok_or_else(|| invalid("invalid activity timestamp"))
}

fn lightning_request(entry: &rpc::WalletEntry) -> Option<&rpc::LightningInvoiceRequest> {
    match entry.request.as_ref()?.request.as_ref()? {
        rpc::wallet_entry_request::Request::LightningInvoice(request) => Some(request),
        _ => None,
    }
}

fn validate_invoice(entry: &rpc::WalletEntry, hash: &str) -> Result<Bolt11Invoice, LightningError> {
    let request = lightning_request(entry).ok_or_else(|| invalid("activity lacks its Lightning request"))?;
    let invoice = Bolt11Invoice::from_str(&request.invoice).map_err(|_| invalid("activity invoice is invalid"))?;
    if invoice.payment_hash().to_string() != hash
        || (!request.payment_hash.is_empty() && request.payment_hash != hash)
        || entry
            .progress
            .as_ref()
            .is_some_and(|p| !p.payment_hash.is_empty() && p.payment_hash != hash)
    {
        return Err(invalid("activity payment hashes disagree"));
    }
    Ok(invoice)
}

#[async_trait]
impl LnClient for WavelengthClient {
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
        if expiry.is_some() || deschashonly {
            return Err(LightningError::Unsupported("waved does not support requested invoice expiry or LNURL description hashes; use invoice_expiry=provider for ordinary BOLT11 receives"));
        }
        let amt_sat = Self::amount_sat(amount_msat)?;
        let response = self
            .client
            .recv(rpc::RecvRequest {
                amt_sat,
                memo: description.clone(),
            })
            .await
            .map_err(|e| LightningError::Invoice(e.to_string()))?;
        if response.credit_receive.is_some() && !self.config.allow_server_credit {
            return Err(LightningError::Unsupported(
                "this Wavelength receive requires explicitly enabling operator credit",
            ));
        }
        let parsed = Bolt11Invoice::from_str(&response.invoice).map_err(|e| LightningError::Invoice(e.to_string()))?;
        if parsed.amount_milli_satoshis() != Some(amount_msat) || parsed.description().to_string() != description {
            return Err(LightningError::Invoice("waved returned a mismatching invoice".into()));
        }
        let expected_network = match parsed.currency() {
            lightning_invoice::Currency::Bitcoin => BtcNetwork::Bitcoin,
            lightning_invoice::Currency::BitcoinTestnet => BtcNetwork::Testnet,
            lightning_invoice::Currency::Regtest => BtcNetwork::Regtest,
            lightning_invoice::Currency::Simnet => BtcNetwork::Simnet,
            lightning_invoice::Currency::Signet => BtcNetwork::Signet,
        };
        if expected_network != self.network
            && !(expected_network == BtcNetwork::Testnet && self.network == BtcNetwork::Testnet4)
        {
            return Err(LightningError::Invoice(
                "waved returned an invoice for another network".into(),
            ));
        }
        let entry = response
            .entry
            .ok_or_else(|| LightningError::Invoice("missing receive activity".into()))?;
        validate_invoice(&entry, &parsed.payment_hash().to_string())?;
        if entry.kind != rpc::EntryKind::Recv as i32 {
            return Err(LightningError::Invoice("wrong receive activity kind".into()));
        }
        Ok(invoice_from_bolt11(parsed))
    }

    async fn estimate_fee(&self, target: LnPaymentTarget) -> Result<u64, LightningError> {
        let principal = Self::amount_sat(target.amount_msat)?;
        let cap = self.fee_cap_sat(self.config.fee_limit_msat)?;
        let invoice =
            Bolt11Invoice::from_str(&target.bolt11).map_err(|e| LightningError::InvalidQuote(e.to_string()))?;
        if invoice.amount_milli_satoshis() != Some(target.amount_msat) {
            return Err(LightningError::Unsupported(
                "waved requires amount-bearing invoices with exact whole-satoshi amounts",
            ));
        }
        let hash = invoice.payment_hash().to_string();
        let quote = self
            .client
            .quote_pay(swaps::QuotePayRequest {
                invoice: target.bolt11,
                max_fee_sat: cap,
                max_credit_sat: if self.config.allow_server_credit {
                    principal.checked_add(cap).ok_or_else(|| invalid("quote overflow"))?
                } else {
                    0
                },
            })
            .await
            .map_err(|e| LightningError::EstimateFee(e.to_string()))?;
        if quote.exceeds_max_fee {
            return Err(LightningError::InvalidQuote("quoted fee exceeds its cap".into()));
        }
        let credit = quote.credit_quote.as_ref();
        self.validate_quote_values(
            QuoteValues {
                hash: &quote.payment_hash,
                principal: quote.invoice_amount_sat,
                fee: quote.fee_sat,
                ark_outflow: quote.amount_sat,
                expires: quote.expires_at_unix,
                credit_applied: credit.map_or(0, |c| c.credit_applied_sat),
                credit_shortfall: credit.map_or(0, |c| c.credit_shortfall_sat),
                credit_topup: credit.map_or(0, |c| c.credit_topup_sat),
            },
            &hash,
            principal,
            cap,
        )
    }
    fn fee_limit_msat(&self, _amount_msat: u64) -> u64 {
        self.config.fee_limit_msat
    }

    async fn pay(
        &self,
        bolt11: String,
        amount_msat: Option<u64>,
        fee_limit_msat: u64,
        label: String,
    ) -> Result<Payment, LightningError> {
        let invoice = Bolt11Invoice::from_str(&bolt11).map_err(|e| LightningError::Pay(e.to_string()))?;
        let amount = invoice.amount_milli_satoshis().ok_or(LightningError::Unsupported(
            "waved requires amount-bearing BOLT11 invoices",
        ))?;
        if amount_msat.is_some_and(|specified| specified != amount) {
            return Err(LightningError::Pay("invoice amount mismatch".into()));
        }
        let principal = Self::amount_sat(amount)?;
        let cap = self.fee_cap_sat(fee_limit_msat)?;
        let hash = invoice.payment_hash().to_string();
        if let Some(existing) = self.payment_by_hash(hash.clone()).await? {
            return match existing {
                payment if payment.status == PaymentStatus::Settled => Ok(payment),
                payment if payment.status == PaymentStatus::Failed => Err(LightningError::Pay(
                    "waved already recorded a failed send for this invoice".into(),
                )),
                _ => Err(LightningError::PaymentOutcomeUnknown(
                    "waved already has this payment; reconcile without resubmitting".into(),
                )),
            };
        }
        let maximum = principal
            .checked_add(cap)
            .ok_or_else(|| LightningError::Pay("amount overflow".into()))?;
        let quote = self
            .client
            .prepare_send(rpc::PrepareSendRequest {
                destination: Some(rpc::prepare_send_request::Destination::Invoice(bolt11)),
                max_fee_sat: cap,
                note: label,
                max_credit_sat: if self.config.allow_server_credit { maximum } else { 0 },
                // Never move additional Ark value into operator credit implicitly.
                max_credit_topup_sat: 0,
                ..Default::default()
            })
            .await
            .map_err(|e| LightningError::Pay(e.to_string()))?;
        self.validate_quote(&quote, &hash, principal, cap)?;
        // The caller has persisted the hash/reservation before entering pay.
        // Send consumes the intent before dispatch. Any failure after this call
        // may follow a payment; do not prepare/send again or release funds.
        let _reply = self
            .client
            .send(rpc::SendRequest {
                send_intent_id: quote.send_intent_id,
            })
            .await;
        let deadline = Instant::now() + self.config.payment_timeout;
        loop {
            match tokio::time::timeout_at(deadline, self.payment_by_hash(hash.clone())).await {
                Ok(Ok(Some(payment))) if payment.status == PaymentStatus::Settled => {
                    if payment.fee_msat.is_none_or(|fee| fee > fee_limit_msat) {
                        return Err(LightningError::PaymentOutcomeUnknown(
                            "settled fee exceeds its authorization; manual reconciliation required".into(),
                        ));
                    }
                    return Ok(payment);
                }
                Ok(Ok(Some(payment))) if payment.status == PaymentStatus::Failed => {
                    return Err(LightningError::Pay(
                        payment
                            .error
                            .unwrap_or_else(|| "waved recorded a failed payment".into()),
                    ))
                }
                Ok(Err(error)) => return Err(LightningError::PaymentOutcomeUnknown(error.to_string())),
                Err(_) => {
                    return Err(LightningError::PaymentOutcomeUnknown(
                        "waved send deadline elapsed; reconcile activity by payment hash".into(),
                    ))
                }
                _ => {}
            }
            if Instant::now() >= deadline {
                return Err(LightningError::PaymentOutcomeUnknown(
                    "waved has not recorded a terminal send outcome".into(),
                ));
            }
            tokio::time::sleep_until((Instant::now() + self.config.poll_interval).min(deadline)).await;
        }
    }

    async fn invoice_by_hash(&self, payment_hash: String) -> Result<Option<Invoice>, LightningError> {
        let Some(entry) = self.find(&payment_hash, rpc::EntryKind::Recv).await? else {
            return Ok(None);
        };
        let parsed = validate_invoice(&entry, &payment_hash)?;
        let mut invoice = invoice_from_bolt11(parsed);
        match rpc::EntryStatus::try_from(entry.status).ok() {
            Some(rpc::EntryStatus::Complete) => {
                let received = u64::try_from(entry.amount_sat)
                    .ok()
                    .and_then(|a| a.checked_mul(1000))
                    .ok_or_else(|| invalid("invalid received amount"))?;
                invoice.status = InvoiceStatus::Settled;
                invoice.amount_received_msat = Some(received);
                invoice.fee_msat = Some(
                    u64::try_from(entry.fee_sat)
                        .ok()
                        .and_then(|a| a.checked_mul(1000))
                        .ok_or_else(|| invalid("invalid receive fee"))?,
                );
                invoice.payment_time = Some(timestamp(entry.updated_at_unix)?);
            }
            Some(rpc::EntryStatus::Pending) => {}
            Some(rpc::EntryStatus::Failed) => invoice.status = InvoiceStatus::Expired,
            _ => return Err(invalid("unknown receive status")),
        }
        Ok(Some(invoice))
    }
    async fn payment_by_hash(&self, payment_hash: String) -> Result<Option<Payment>, LightningError> {
        match self
            .client
            .get_swap(swaps::GetSwapRequest {
                payment_hash: payment_hash.clone(),
            })
            .await
        {
            Ok(response) => {
                let summary = response.swap.ok_or_else(|| invalid("missing persisted swap summary"))?;
                Ok(Some(self.payment_from_swap(summary, &payment_hash).await?))
            }
            Err(Error::Rpc(status)) if status.code() == tonic::Code::NotFound => Ok(None),
            Err(error) => Err(LightningError::PaymentByHash(error.to_string())),
        }
    }
    async fn cancel_invoice(&self, _payment_hash: String, _label: String) -> Result<(), LightningError> {
        Err(LightningError::Unsupported(
            "waved does not expose ordinary invoice cancellation",
        ))
    }
    async fn health(&self) -> Result<HealthStatus, LightningError> {
        let status = self
            .client
            .status(Default::default())
            .await
            .map_err(|e| LightningError::HealthCheck(e.to_string()))?;
        Ok(if status.ready {
            HealthStatus::Operational
        } else {
            HealthStatus::Unavailable
        })
    }
}
