//! Independent implementation of the public wallet RPC contract for failure
//! injection. This is not an Ark operator and never claims funded coverage.
use bitcoin::{
    hashes::{sha256, Hash},
    secp256k1::{Secp256k1, SecretKey},
};
use lightning_invoice::{Bolt11Invoice, Currency, InvoiceBuilder, PaymentSecret};
use std::{
    collections::HashMap,
    pin::Pin,
    sync::{Arc, Mutex},
    time::{Duration, SystemTime},
};
use tokio::{net::TcpListener, task::JoinHandle};
use tokio_stream::{wrappers::TcpListenerStream, Stream};
use tonic::{
    transport::{Identity, Server, ServerTlsConfig},
    Request, Response, Status,
};
use wavelength_client::rpc::{
    self,
    wallet_service_server::{WalletService, WalletServiceServer},
};
use wavelength_client::swaps::{
    self,
    swap_client_service_server::{SwapClientService, SwapClientServiceServer},
};

pub const CERT: &[u8] = include_bytes!("../../crates/wavelength-client/tests/fixtures/cert.pem");
const KEY: &[u8] = include_bytes!("../../crates/wavelength-client/tests/fixtures/key.pem");
pub const TOKEN: &[u8] = b"public-protocol-test-only";

#[derive(Clone, Copy, Debug, Default)]
pub enum Outcome {
    #[default]
    LostReply,
    Complete,
    WalletTimeout,
    Refunded,
    OverBudget,
    BadProof,
}

#[derive(Clone, Copy, Debug, Default)]
pub enum Quote {
    #[default]
    Complete,
    Partial,
    WrongHash,
    OverBudget,
    CreditTopup,
    CreditOnly,
    ReadOnlyWrongHash,
}

#[derive(Default)]
pub struct State {
    pub entries: Vec<rpc::WalletEntry>,
    intents: HashMap<String, String>,
    pub prepares: Vec<rpc::PrepareSendRequest>,
    pub sends: usize,
    pub receives: usize,
    pub outcome: Outcome,
    pub quote: Quote,
    canonical_fees: HashMap<String, u64>,
    pub credit_settled: bool,
    pub hidden_sends: bool,
}

impl State {
    pub fn intents_clear(&mut self) {
        self.intents.clear();
    }
}

#[derive(Clone, Default)]
pub struct Daemon(pub Arc<Mutex<State>>);

pub fn invoice(amount_msat: u64, memo: &str, preimage: [u8; 32]) -> Bolt11Invoice {
    let key = SecretKey::from_slice(&[42; 32]).unwrap();
    InvoiceBuilder::new(Currency::Regtest)
        .description(memo.into())
        .payment_hash(sha256::Hash::hash(&preimage))
        .payment_secret(PaymentSecret([7; 32]))
        .duration_since_epoch(SystemTime::now().duration_since(SystemTime::UNIX_EPOCH).unwrap())
        .min_final_cltv_expiry_delta(18)
        .expiry_time(Duration::from_secs(3600))
        .amount_milli_satoshis(amount_msat)
        .build_signed(|message| Secp256k1::new().sign_ecdsa_recoverable(message, &key))
        .unwrap()
}

fn entry(invoice: &Bolt11Invoice, kind: rpc::EntryKind) -> rpc::WalletEntry {
    let hash = invoice.payment_hash().to_string();
    let now = chrono::Utc::now().timestamp();
    rpc::WalletEntry {
        id: if kind == rpc::EntryKind::Recv {
            format!("credit-{}", uuid::Uuid::new_v4())
        } else {
            hash.clone()
        },
        kind: kind as i32,
        status: rpc::EntryStatus::Pending as i32,
        amount_sat: (invoice.amount_milli_satoshis().unwrap() / 1000) as i64
            * if kind == rpc::EntryKind::Recv { 1 } else { -1 },
        created_at_unix: now,
        updated_at_unix: now,
        request: Some(rpc::WalletEntryRequest {
            request: Some(rpc::wallet_entry_request::Request::LightningInvoice(
                rpc::LightningInvoiceRequest {
                    invoice: invoice.to_string(),
                    payment_hash: hash.clone(),
                },
            )),
        }),
        progress: Some(rpc::WalletEntryProgress {
            payment_hash: hash,
            phase: rpc::WalletEntryPhase::WaitingForPayment as i32,
            ..Default::default()
        }),
        ..Default::default()
    }
}

impl Daemon {
    pub fn record_existing_send(&self, invoice: &Bolt11Invoice) {
        let mut state = self.0.lock().unwrap();
        state.canonical_fees.insert(invoice.payment_hash().to_string(), 2);
        state.entries.push(entry(invoice, rpc::EntryKind::Send));
        state.hidden_sends = true;
    }
    pub fn complete(&self, hash: &str, preimage: Option<[u8; 32]>, fee_sat: i64) {
        let mut state = self.0.lock().unwrap();
        let entry = state
            .entries
            .iter_mut()
            .find(|e| e.progress.as_ref().unwrap().payment_hash == hash)
            .unwrap();
        entry.status = rpc::EntryStatus::Complete as i32;
        entry.failure_code = None;
        entry.fee_sat = fee_sat;
        if entry.kind == rpc::EntryKind::Send as i32 {
            entry.amount_sat = -(entry.amount_sat.abs() + fee_sat);
        }
        let progress = entry.progress.as_mut().unwrap();
        progress.phase = rpc::WalletEntryPhase::Confirmed as i32;
        progress.preimage = preimage.map(hex::encode).unwrap_or_default();
    }

    pub async fn serve(&self) -> (String, JoinHandle<()>) {
        let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
        let endpoint = format!("https://localhost:{}", listener.local_addr().unwrap().port());
        let service = WalletServiceServer::with_interceptor(self.clone(), |request: Request<()>| {
            if request.metadata().get("macaroon").and_then(|v| v.to_str().ok()) != Some(hex::encode(TOKEN).as_str()) {
                return Err(Status::unauthenticated("test macaroon required"));
            }
            Ok(request)
        });
        let daemon = self.clone();
        let task = tokio::spawn(async move {
            Server::builder()
                .tls_config(ServerTlsConfig::new().identity(Identity::from_pem(CERT, KEY)))
                .unwrap()
                .add_service(service)
                .add_service(SwapClientServiceServer::with_interceptor(
                    daemon,
                    |request: Request<()>| {
                        if request.metadata().get("macaroon").and_then(|v| v.to_str().ok())
                            != Some(hex::encode(TOKEN).as_str())
                        {
                            return Err(Status::unauthenticated("test macaroon required"));
                        }
                        Ok(request)
                    },
                ))
                .serve_with_incoming(TcpListenerStream::new(listener))
                .await
                .unwrap();
        });
        (endpoint, task)
    }
}

#[tonic::async_trait]
impl SwapClientService for Daemon {
    async fn quote_pay(
        &self,
        request: Request<swaps::QuotePayRequest>,
    ) -> Result<Response<swaps::QuotePayResponse>, Status> {
        let input = request.into_inner();
        assert_eq!(input.max_fee_sat, 25);
        let invoice = input.invoice.parse::<Bolt11Invoice>().unwrap();
        let principal = invoice.amount_milli_satoshis().unwrap() / 1000;
        let mode = self.0.lock().unwrap().quote;
        let credit = matches!(mode, Quote::CreditOnly);
        Ok(Response::new(swaps::QuotePayResponse {
            payment_hash: if matches!(mode, Quote::ReadOnlyWrongHash) {
                "wrong hash".into()
            } else {
                invoice.payment_hash().to_string()
            },
            invoice_amount_sat: principal,
            amount_sat: if credit { 0 } else { principal + 2 },
            fee_sat: 2,
            expires_at_unix: chrono::Utc::now().timestamp() + 120,
            settlement_type: if credit {
                swaps::SwapSettlementType::Credit
            } else {
                swaps::SwapSettlementType::Lightning
            } as i32,
            credit_quote: credit.then_some(swaps::CreditQuote {
                must_use_credit: true,
                credit_applied_sat: principal + 2,
                ..Default::default()
            }),
            ..Default::default()
        }))
    }
    async fn get_swap(
        &self,
        request: Request<swaps::GetSwapRequest>,
    ) -> Result<Response<swaps::GetSwapResponse>, Status> {
        let hash = request.into_inner().payment_hash;
        let state = self.0.lock().unwrap();
        let entry = state
            .entries
            .iter()
            .find(|e| e.kind == rpc::EntryKind::Send as i32 && e.progress.as_ref().unwrap().payment_hash == hash)
            .ok_or_else(|| Status::not_found("no persisted send"))?;
        let Some(rpc::wallet_entry_request::Request::LightningInvoice(request)) =
            entry.request.as_ref().unwrap().request.as_ref()
        else {
            unreachable!()
        };
        let credit = matches!(state.quote, Quote::CreditOnly);
        let completed = credit || entry.status == rpc::EntryStatus::Complete as i32;
        let refunded = entry.failure_code == Some(rpc::EntryFailureCode::Refunded as i32);
        Ok(Response::new(swaps::GetSwapResponse {
            swap: Some(swaps::SwapSummary {
                payment_hash: hash,
                invoice: request.invoice.clone(),
                direction: swaps::SwapDirection::Pay as i32,
                pending: !completed && !refunded,
                created_at_unix: entry.created_at_unix,
                updated_at_unix: entry.updated_at_unix,
                state: if refunded {
                    swaps::SwapState::Refunded
                } else if completed {
                    swaps::SwapState::Completed
                } else {
                    swaps::SwapState::WaitingForClaim
                } as i32,
                fee_sat: *state.canonical_fees.get(&request.payment_hash).unwrap_or(&0),
                settlement_type: if credit {
                    swaps::SwapSettlementType::Credit
                } else {
                    swaps::SwapSettlementType::Lightning
                } as i32,
                preimage: if credit {
                    hex::encode([21; 32])
                } else {
                    entry.progress.as_ref().unwrap().preimage.clone()
                },
                ..Default::default()
            }),
        }))
    }
    async fn list_credits(
        &self,
        _: Request<swaps::ListCreditsRequest>,
    ) -> Result<Response<swaps::ListCreditsResponse>, Status> {
        let state = self.0.lock().unwrap();
        Ok(Response::new(swaps::ListCreditsResponse {
            operations: state
                .entries
                .iter()
                .filter(|e| e.kind == rpc::EntryKind::Send as i32)
                .map(|entry| swaps::CreditOperation {
                    payment_hash: entry.progress.as_ref().unwrap().payment_hash.clone(),
                    r#type: swaps::CreditOperationType::Pay as i32,
                    state: if state.credit_settled {
                        swaps::CreditOperationState::Debited
                    } else {
                        swaps::CreditOperationState::PayingLightning
                    } as i32,
                    ..Default::default()
                })
                .collect(),
            ..Default::default()
        }))
    }
    async fn start_pay(&self, _: Request<swaps::StartPayRequest>) -> Result<Response<swaps::StartPayResponse>, Status> {
        Err(Status::unimplemented("unused"))
    }
    async fn start_receive(
        &self,
        _: Request<swaps::StartReceiveRequest>,
    ) -> Result<Response<swaps::StartReceiveResponse>, Status> {
        Err(Status::unimplemented("unused"))
    }
    async fn create_credit(
        &self,
        _: Request<swaps::CreateCreditRequest>,
    ) -> Result<Response<swaps::CreateCreditResponse>, Status> {
        Err(Status::unimplemented("unused"))
    }
    async fn redeem_credit(
        &self,
        _: Request<swaps::RedeemCreditRequest>,
    ) -> Result<Response<swaps::RedeemCreditResponse>, Status> {
        Err(Status::unimplemented("unused"))
    }
    async fn resume_swap(
        &self,
        _: Request<swaps::ResumeSwapRequest>,
    ) -> Result<Response<swaps::ResumeSwapResponse>, Status> {
        Err(Status::unimplemented("unused"))
    }
    async fn list_swaps(
        &self,
        _: Request<swaps::ListSwapsRequest>,
    ) -> Result<Response<swaps::ListSwapsResponse>, Status> {
        Err(Status::unimplemented("unused"))
    }
    type SubscribeSwapsStream = Pin<Box<dyn Stream<Item = Result<swaps::SubscribeSwapsResponse, Status>> + Send>>;
    async fn subscribe_swaps(
        &self,
        _: Request<swaps::SubscribeSwapsRequest>,
    ) -> Result<Response<Self::SubscribeSwapsStream>, Status> {
        Err(Status::unimplemented("unused"))
    }
}

#[tonic::async_trait]
impl WalletService for Daemon {
    async fn create(&self, _: Request<rpc::CreateRequest>) -> Result<Response<rpc::CreateResponse>, Status> {
        Err(Status::unimplemented("outside the failure-injection contract"))
    }
    async fn unlock(&self, _: Request<rpc::UnlockRequest>) -> Result<Response<rpc::UnlockResponse>, Status> {
        Err(Status::unimplemented("outside the failure-injection contract"))
    }
    async fn deposit(&self, _: Request<rpc::DepositRequest>) -> Result<Response<rpc::DepositResponse>, Status> {
        Err(Status::unimplemented("outside the failure-injection contract"))
    }
    async fn get_exit_plan(
        &self,
        _: Request<rpc::GetExitPlanRequest>,
    ) -> Result<Response<rpc::GetExitPlanResponse>, Status> {
        Err(Status::unimplemented("outside the failure-injection contract"))
    }
    async fn sweep_wallet(
        &self,
        _: Request<rpc::SweepWalletRequest>,
    ) -> Result<Response<rpc::SweepWalletResponse>, Status> {
        Err(Status::unimplemented("outside the failure-injection contract"))
    }
    async fn exit(&self, _: Request<rpc::ExitRequest>) -> Result<Response<rpc::ExitResponse>, Status> {
        Err(Status::unimplemented("outside the failure-injection contract"))
    }
    async fn exit_status(
        &self,
        _: Request<rpc::ExitStatusRequest>,
    ) -> Result<Response<rpc::ExitStatusResponse>, Status> {
        Err(Status::unimplemented("outside the failure-injection contract"))
    }
    async fn exit_summary(
        &self,
        _: Request<rpc::ExitSummaryRequest>,
    ) -> Result<Response<rpc::ExitSummaryResponse>, Status> {
        Err(Status::unimplemented("outside the failure-injection contract"))
    }

    async fn recv(&self, request: Request<rpc::RecvRequest>) -> Result<Response<rpc::RecvResponse>, Status> {
        let input = request.into_inner();
        let mut preimage = [0; 32];
        preimage[..16].copy_from_slice(uuid::Uuid::new_v4().as_bytes());
        let invoice = invoice(input.amt_sat * 1000, &input.memo, preimage);
        let entry = entry(&invoice, rpc::EntryKind::Recv);
        let mut state = self.0.lock().unwrap();
        state.receives += 1;
        state.entries.push(entry.clone());
        Ok(Response::new(rpc::RecvResponse {
            invoice: invoice.to_string(),
            credit_receive: Some(rpc::CreditReceive {
                operation_id: entry.id.clone(),
                amount_sat: input.amt_sat,
                payment_hash: invoice.payment_hash().to_string(),
            }),
            entry: Some(entry),
        }))
    }

    async fn prepare_send(
        &self,
        request: Request<rpc::PrepareSendRequest>,
    ) -> Result<Response<rpc::PrepareSendResponse>, Status> {
        let input = request.into_inner();
        assert_eq!(input.max_fee_sat, 25);
        assert_eq!(input.max_credit_topup_sat, 0);
        let Some(rpc::prepare_send_request::Destination::Invoice(bolt11)) = input.destination.as_ref() else {
            return Err(Status::invalid_argument("invoice required"));
        };
        let invoice = bolt11.parse::<Bolt11Invoice>().unwrap();
        let amount = (invoice.amount_milli_satoshis().unwrap() / 1000) as i64;
        let id = uuid::Uuid::new_v4().to_string();
        let mut state = self.0.lock().unwrap();
        state.intents.insert(id.clone(), bolt11.clone());
        let mut response = rpc::PrepareSendResponse {
            send_intent_id: id,
            amount_sat: amount,
            expected_fee_sat: 2,
            fee_known: true,
            expected_total_outflow_sat: amount + 2,
            total_outflow_known: true,
            quote_status: rpc::SendQuoteStatus::Complete as i32,
            payment_hash: invoice.payment_hash().to_string(),
            expires_at_unix: chrono::Utc::now().timestamp() + 120,
            ..Default::default()
        };
        match state.quote {
            Quote::Complete | Quote::ReadOnlyWrongHash => {}
            Quote::CreditOnly => {
                response.expected_total_outflow_sat = 0;
                response.credit_preview = Some(rpc::CreditPreview {
                    must_use_credit: true,
                    credit_applied_sat: (amount + 2) as u64,
                    ..Default::default()
                });
            }
            Quote::Partial => {
                response.fee_known = false;
                response.quote_status = rpc::SendQuoteStatus::LocalOnly as i32;
            }
            Quote::WrongHash => response.payment_hash = "wrong hash".into(),
            Quote::OverBudget => {
                response.expected_fee_sat = 26;
                response.expected_total_outflow_sat = amount + 26;
            }
            Quote::CreditTopup => {
                response.credit_preview = Some(rpc::CreditPreview {
                    credit_topup_sat: 100_000,
                    ..Default::default()
                })
            }
        }
        state.prepares.push(input);
        Ok(Response::new(response))
    }

    async fn send(&self, request: Request<rpc::SendRequest>) -> Result<Response<rpc::SendResponse>, Status> {
        let mut state = self.0.lock().unwrap();
        state.sends += 1;
        let bolt11 = state
            .intents
            .remove(&request.into_inner().send_intent_id)
            .ok_or_else(|| Status::failed_precondition("consumed intent"))?;
        let invoice: Bolt11Invoice = bolt11.parse().unwrap();
        let mut entry = entry(&invoice, rpc::EntryKind::Send);
        match state.outcome {
            Outcome::LostReply => {}
            Outcome::WalletTimeout => {
                entry.status = rpc::EntryStatus::Failed as i32;
                entry.failure_code = Some(rpc::EntryFailureCode::TimedOut as i32);
            }
            Outcome::Refunded => {
                entry.status = rpc::EntryStatus::Failed as i32;
                entry.failure_code = Some(rpc::EntryFailureCode::Refunded as i32);
            }
            Outcome::Complete | Outcome::OverBudget | Outcome::BadProof => {
                entry.status = rpc::EntryStatus::Complete as i32;
                entry.fee_sat = if matches!(state.outcome, Outcome::OverBudget) {
                    26
                } else {
                    2
                };
                entry.amount_sat -= entry.fee_sat;
                entry.progress.as_mut().unwrap().preimage =
                    hex::encode(if matches!(state.outcome, Outcome::BadProof) {
                        [99; 32]
                    } else {
                        [21; 32]
                    });
            }
        }
        let fee = if matches!(state.outcome, Outcome::OverBudget) {
            26
        } else {
            2
        };
        state.canonical_fees.insert(invoice.payment_hash().to_string(), fee);
        if matches!(state.quote, Quote::CreditOnly) {
            entry.counterparty = "credit".into();
            entry.fee_sat = 0;
            entry.amount_sat = -((invoice.amount_milli_satoshis().unwrap() / 1000) as i64);
            entry.failure_code = Some(rpc::EntryFailureCode::Failed as i32);
        }
        state.entries.push(entry);
        Err(Status::unavailable("reply lost after accepting the send"))
    }

    async fn list(&self, request: Request<rpc::ListRequest>) -> Result<Response<rpc::ListResponse>, Status> {
        let input = request.into_inner();
        let start: usize = if input.cursor.is_empty() {
            0
        } else {
            input.cursor.parse().unwrap()
        };
        let state = self.0.lock().unwrap();
        let entries: Vec<_> = state
            .entries
            .iter()
            .filter(|entry| !state.hidden_sends || entry.kind != rpc::EntryKind::Send as i32)
            .cloned()
            .collect();
        let end = (start + input.limit as usize).min(entries.len());
        let has_more = end < entries.len();
        Ok(Response::new(rpc::ListResponse {
            body: Some(rpc::list_response::Body::Activity(rpc::ActivityList {
                entries: entries[start..end].to_vec(),
                has_more,
                next_cursor: if has_more { end.to_string() } else { String::new() },
                total: entries.len() as u32,
            })),
        }))
    }
    async fn balance(&self, _: Request<rpc::BalanceRequest>) -> Result<Response<rpc::BalanceResponse>, Status> {
        Ok(Response::new(Default::default()))
    }
    async fn status(&self, _: Request<rpc::StatusRequest>) -> Result<Response<rpc::StatusResponse>, Status> {
        Ok(Response::new(rpc::StatusResponse {
            ready: true,
            unlocked: true,
            network: "regtest".into(),
            balance: Some(Default::default()),
            ..Default::default()
        }))
    }
    type SubscribeWalletStream = Pin<Box<dyn Stream<Item = Result<rpc::SubscribeWalletResponse, Status>> + Send>>;
    async fn subscribe_wallet(
        &self,
        _: Request<rpc::SubscribeWalletRequest>,
    ) -> Result<Response<Self::SubscribeWalletStream>, Status> {
        Err(Status::unimplemented("polling reconciliation contract"))
    }
}
