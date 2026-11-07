//! Deterministic CLN wire peer for black-box payment lifecycle regressions.
//! Funding uses the real regtest stack before SwissKnife restarts against this
//! peer. The application is always the compiled binary, accessed by public API.
use axum::{
    extract::{
        ws::{Message, WebSocket},
        Path, State, WebSocketUpgrade,
    },
    http::StatusCode,
    response::IntoResponse,
    routing::{get, post},
    serve, Json, Router,
};
use bitcoin::{
    hashes::{sha256, Hash},
    secp256k1::{PublicKey, Secp256k1, SecretKey},
};
use futures_util::stream::unfold;
use lightning_invoice::{Currency, InvoiceBuilder, PaymentSecret, RouteHint, RouteHintHop, RoutingFees};
use serde_json::{json, Value};
use std::{
    fs::read,
    future::pending,
    path::Path as FsPath,
    sync::{
        atomic::{AtomicBool, AtomicU64, AtomicUsize, Ordering},
        Arc, Mutex,
    },
    time::{Duration, SystemTime, UNIX_EPOCH},
};
use tokio::{
    net::TcpListener,
    spawn,
    sync::{broadcast, watch},
    task::JoinHandle,
    time::interval,
};
use tonic::{
    transport::{Identity, Server, ServerTlsConfig},
    Request, Response, Status,
};

#[allow(dead_code, clippy::all)]
pub mod pb {
    tonic::include_proto!("cln");
}

pub const AMOUNT: u64 = 1_000_000;
pub const RESERVED: u64 = 1_010_000;
pub const FINAL_FEE: u64 = 5_000;
const PROOF: [u8; 32] = [21; 32];

#[derive(Clone, Copy)]
pub enum Outcome {
    Pending = 0,
    LookupError = 1,
    Partial = 2,
    Complete = 3,
    Failed = 4,
    Missing = 5,
    Incomplete = 6,
    LookupTimeout = 7,
}

pub struct Scenario {
    pub outcome: AtomicUsize,
    pub sends: AtomicUsize,
    pub rpc_success: AtomicBool,
    pub lookups: AtomicUsize,
    pub quotes: Mutex<Vec<Value>>,
    pub hash: String,
    events: broadcast::Sender<Value>,
    updates: watch::Sender<(u64, i32)>,
    sequence: AtomicU64,
}

pub struct Peer {
    pub scenario: Arc<Scenario>,
    pub env: Vec<(&'static str, String)>,
    task: JoinHandle<()>,
}
impl Drop for Peer {
    fn drop(&mut self) {
        self.task.abort();
    }
}

fn key(n: u8) -> PublicKey {
    PublicKey::from_secret_key(&Secp256k1::new(), &SecretKey::from_slice(&[n; 32]).unwrap())
}

pub fn invoice(seed: u8, private: bool) -> String {
    let secret = SecretKey::from_slice(&[3; 32]).unwrap();
    let mut builder = InvoiceBuilder::new(Currency::Regtest)
        .amount_milli_satoshis(AMOUNT)
        .description("CLN reconciliation regression".to_string())
        .payment_hash(sha256::Hash::hash(&[seed; 32]))
        .payment_secret(PaymentSecret([31; 32]))
        .duration_since_epoch(SystemTime::now().duration_since(UNIX_EPOCH).unwrap())
        .min_final_cltv_expiry_delta(40);
    if private {
        builder = builder.private_route(RouteHint(vec![RouteHintHop {
            src_node_id: key(2),
            short_channel_id: 12345,
            fees: RoutingFees {
                base_msat: 1_000,
                proportional_millionths: 2_500,
            },
            cltv_expiry_delta: 144,
            htlc_minimum_msat: None,
            htlc_maximum_msat: None,
        }]));
    }
    builder
        .build_signed(|hash| Secp256k1::new().sign_ecdsa_recoverable(hash, &secret))
        .unwrap()
        .to_string()
}

impl Scenario {
    pub fn set(&self, outcome: Outcome) {
        self.outcome.store(outcome as usize, Ordering::SeqCst);
    }
    pub fn notify(&self, status: &str) {
        let n = self.sequence.fetch_add(1, Ordering::SeqCst) + 1;
        let code = if status == "complete" { 2 } else { 1 };
        self.updates.send_replace((n, code));
        let event = if status == "complete" {
            json!({"sendpay_success":{"status":"complete","payment_hash":self.hash,"amount_msat":600000,"amount_sent_msat":602000,"payment_preimage":hex::encode(PROOF),"completed_at":1700000000}})
        } else {
            json!({"sendpay_failure":{"code":202,"message":"Malformed error reply","data":{"status":status,"payment_hash":self.hash,"partid":1,"groupid":1}}})
        };
        let _ = self.events.send(event);
    }
    async fn summary(&self) -> Result<Option<&'static str>, Status> {
        self.lookups.fetch_add(1, Ordering::SeqCst);
        if self.sends.load(Ordering::SeqCst) == 0 {
            return Ok(None);
        }
        match self.outcome.load(Ordering::SeqCst) {
            0 => Ok(Some("pending")),
            1 => Err(Status::unavailable("lookup unavailable")),
            2 | 3 | 6 => Ok(Some("complete")),
            4 => Ok(Some("failed")),
            5 => Ok(None),
            7 => pending().await,
            _ => unreachable!(),
        }
    }
    fn parts(&self) -> Vec<Value> {
        let outcome = self.outcome.load(Ordering::SeqCst);
        let partial = outcome == Outcome::Partial as usize;
        let final_part_status = if partial {
            "pending"
        } else if outcome == Outcome::Incomplete as usize {
            "failed"
        } else {
            "complete"
        };
        [
            ("failed", AMOUNT, AMOUNT + 9_000),
            ("complete", 600_000, 602_000),
            (final_part_status, 400_000, 403_000),
        ]
        .into_iter()
        .map(|(status, amount, sent)| {
            json!({
                "payment_hash": self.hash,
                "status": status,
                "amount_msat": amount,
                "amount_sent_msat": sent,
                "payment_preimage": hex::encode(PROOF),
                "completed_at": 1700000000,
            })
        })
        .collect()
    }

    fn quote(
        &self,
        destination: &str,
        amount: u64,
        cltv: u32,
        maxfee: u64,
        maxparts: Option<u32>,
    ) -> Result<u64, Status> {
        self.quotes.lock().unwrap().push(json!({"destination":destination,"amount_msat":amount,"final_cltv":cltv,"maxfee_msat":maxfee,"maxparts":maxparts}));
        if destination != key(2).to_string() {
            return Err(Status::unknown("Unknown destination node"));
        }
        Ok(1_234)
    }
}

impl Peer {
    pub async fn start(provider: &str) -> Self {
        let (events, _) = broadcast::channel(32);
        let (updates, _) = watch::channel((0, 0));
        let scenario = Arc::new(Scenario {
            outcome: AtomicUsize::new(0),
            sends: AtomicUsize::new(0),
            rpc_success: AtomicBool::new(false),
            lookups: AtomicUsize::new(0),
            quotes: Mutex::new(vec![]),
            hash: sha256::Hash::hash(&PROOF).to_string(),
            events,
            updates,
            sequence: AtomicU64::new(0),
        });
        let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
        let port = listener.local_addr().unwrap().port();
        let mut env = vec![("SWISSKNIFE_WEB__REQUEST_TIMEOUT", "1s".to_string())];
        let task = if provider == "cln_rest" {
            env.push((
                "SWISSKNIFE_CLN_REST_CONFIG__ENDPOINT",
                format!("http://127.0.0.1:{port}"),
            ));
            let app = Router::new()
                .route("/v1/{method}", post(rest))
                .route("/socket.io/", get(socket))
                .with_state(scenario.clone());
            spawn(async move {
                serve(listener, app).await.unwrap();
            })
        } else {
            env.push((
                "SWISSKNIFE_CLN_GRPC_CONFIG__ENDPOINT",
                format!("https://localhost:{port}"),
            ));
            let certs = FsPath::new(env!("CARGO_MANIFEST_DIR")).join("tests/itest/runtime/cln/regtest");
            let identity = Identity::from_pem(
                read(certs.join("server.pem")).unwrap(),
                read(certs.join("server-key.pem")).unwrap(),
            );
            let service = pb::node_server::NodeServer::new(Grpc(scenario.clone()));
            spawn(async move {
                let incoming = unfold(listener, |listener| async move {
                    Some((listener.accept().await.map(|(stream, _)| stream), listener))
                });
                Server::builder()
                    .tls_config(ServerTlsConfig::new().identity(identity))
                    .unwrap()
                    .add_service(service)
                    .serve_with_incoming(incoming)
                    .await
                    .unwrap();
            })
        };
        Self { scenario, env, task }
    }
}

async fn socket(State(s): State<Arc<Scenario>>, ws: WebSocketUpgrade) -> impl IntoResponse {
    ws.on_upgrade(move |ws| socket_loop(s, ws))
}
async fn socket_loop(s: Arc<Scenario>, mut ws: WebSocket) {
    let mut rx = s.events.subscribe();
    if ws
        .send(Message::Text(
            r#"0{"sid":"test","upgrades":[],"pingInterval":25000,"pingTimeout":20000,"maxPayload":1000000}"#.into(),
        ))
        .await
        .is_err()
    {
        return;
    }
    let mut ping = interval(Duration::from_secs(10));
    ping.tick().await;
    loop {
        tokio::select! {
            msg=ws.recv()=>match msg {
                Some(Ok(Message::Text(text))) if text.starts_with("40")=>{ if ws.send(Message::Text("40{\"sid\":\"test\"}".into())).await.is_err(){return;} },
                None|Some(Err(_))|Some(Ok(Message::Close(_)))=>return,
                _=>{},
            },
            event=rx.recv()=>if let Ok(event)=event { if ws.send(Message::Text(format!("42{}",json!(["message",event])).into())).await.is_err(){return;} },
            _=ping.tick()=>{ if ws.send(Message::Text("2".into())).await.is_err(){return;} }
        }
    }
}
async fn rest(
    State(s): State<Arc<Scenario>>,
    Path(method): Path<String>,
    Json(p): Json<Value>,
) -> (StatusCode, Json<Value>) {
    let value = match method.as_str() {
        "getinfo" => json!({"id":key(1).to_string(),"network":"regtest"}),
        "listchainmoves" => json!({"chainmoves":[]}),
        "listinvoices" => json!({"invoices":[]}),
        "listpays" => match s.summary().await {
            Ok(Some(status)) => json!({"pays":[{"payment_hash":s.hash,"status":status}]}),
            Ok(None) => json!({"pays":[]}),
            Err(_) => {
                return (
                    StatusCode::SERVICE_UNAVAILABLE,
                    Json(json!({"message":"lookup unavailable"})),
                )
            }
        },
        "listsendpays" => json!({"payments":s.parts()}),
        "getroutes" => {
            let amount = p["amount_msat"].as_u64().unwrap();
            match s.quote(
                p["destination"].as_str().unwrap(),
                amount,
                p["final_cltv"].as_u64().unwrap() as u32,
                p["maxfee_msat"].as_u64().unwrap(),
                p["maxparts"].as_u64().map(|v| v as u32),
            ) {
                Ok(fee) => json!({"routes":[{"amount_msat":amount,"path":[{"amount_in_msat":amount+fee}]}]}),
                Err(err) => {
                    return (
                        StatusCode::BAD_REQUEST,
                        Json(json!({"message":err.message(),"code":205})),
                    )
                }
            }
        }
        "xpay" => {
            s.sends.fetch_add(1, Ordering::SeqCst);
            if !s.rpc_success.load(Ordering::SeqCst) {
                return pending().await;
            }
            json!({"payment_preimage":hex::encode(PROOF),"amount_msat":AMOUNT,"amount_sent_msat":AMOUNT+FINAL_FEE})
        }
        _ => {
            return (
                StatusCode::NOT_FOUND,
                Json(json!({"message":format!("Unexpected RPC {method}")})),
            )
        }
    };
    (StatusCode::OK, Json(value))
}

struct Grpc(Arc<Scenario>);
#[tonic::async_trait]
impl pb::node_server::Node for Grpc {
    async fn getinfo(&self, _: Request<pb::GetinfoRequest>) -> Result<Response<pb::GetinfoResponse>, Status> {
        Ok(Response::new(pb::GetinfoResponse {
            id: key(1).serialize().to_vec(),
            network: "regtest".to_string(),
            ..Default::default()
        }))
    }
    async fn list_chain_moves(
        &self,
        _: Request<pb::ListchainmovesRequest>,
    ) -> Result<Response<pb::ListchainmovesResponse>, Status> {
        Ok(Response::new(pb::ListchainmovesResponse::default()))
    }
    async fn list_invoices(
        &self,
        _: Request<pb::ListinvoicesRequest>,
    ) -> Result<Response<pb::ListinvoicesResponse>, Status> {
        Ok(Response::new(pb::ListinvoicesResponse::default()))
    }
    async fn list_pays(&self, _: Request<pb::ListpaysRequest>) -> Result<Response<pb::ListpaysResponse>, Status> {
        let pays = self
            .0
            .summary()
            .await?
            .map(|status| pb::ListpaysPays {
                payment_hash: hex::decode(&self.0.hash).unwrap(),
                status: match status {
                    "pending" => 0,
                    "complete" => 2,
                    "failed" => 1,
                    _ => unreachable!(),
                },
                ..Default::default()
            })
            .into_iter()
            .collect();
        Ok(Response::new(pb::ListpaysResponse { pays }))
    }
    async fn list_send_pays(
        &self,
        _: Request<pb::ListsendpaysRequest>,
    ) -> Result<Response<pb::ListsendpaysResponse>, Status> {
        let payments = self
            .0
            .parts()
            .into_iter()
            .map(|v| pb::ListsendpaysPayments {
                payment_hash: hex::decode(&self.0.hash).unwrap(),
                status: match v["status"].as_str().unwrap() {
                    "pending" => 0,
                    "failed" => 1,
                    "complete" => 2,
                    _ => unreachable!(),
                },
                amount_msat: Some(pb::Amount {
                    msat: v["amount_msat"].as_u64().unwrap(),
                }),
                amount_sent_msat: Some(pb::Amount {
                    msat: v["amount_sent_msat"].as_u64().unwrap(),
                }),
                payment_preimage: Some(PROOF.to_vec()),
                completed_at: Some(1700000000),
                ..Default::default()
            })
            .collect();
        Ok(Response::new(pb::ListsendpaysResponse { payments }))
    }
    async fn get_routes(&self, r: Request<pb::GetroutesRequest>) -> Result<Response<pb::GetroutesResponse>, Status> {
        let r = r.into_inner();
        let amount = r.amount_msat.unwrap().msat;
        let fee = self.0.quote(
            &hex::encode(r.destination),
            amount,
            r.final_cltv,
            r.maxfee_msat.unwrap().msat,
            r.maxparts,
        )?;
        Ok(Response::new(pb::GetroutesResponse {
            routes: vec![pb::GetroutesRoutes {
                amount_msat: Some(pb::Amount { msat: amount }),
                path: vec![pb::GetroutesRoutesPath {
                    amount_in_msat: Some(pb::Amount { msat: amount + fee }),
                    ..Default::default()
                }],
                ..Default::default()
            }],
            ..Default::default()
        }))
    }
    async fn xpay(&self, _: Request<pb::XpayRequest>) -> Result<Response<pb::XpayResponse>, Status> {
        self.0.sends.fetch_add(1, Ordering::SeqCst);
        if !self.0.rpc_success.load(Ordering::SeqCst) {
            return pending().await;
        }
        Ok(Response::new(pb::XpayResponse {
            payment_preimage: PROOF.to_vec(),
            amount_msat: Some(pb::Amount { msat: AMOUNT }),
            amount_sent_msat: Some(pb::Amount {
                msat: AMOUNT + FINAL_FEE,
            }),
            ..Default::default()
        }))
    }
    async fn wait(&self, r: Request<pb::WaitRequest>) -> Result<Response<pb::WaitResponse>, Status> {
        let r = r.into_inner();
        if r.subsystem != 2 {
            return pending().await;
        }
        let mut updates = self.0.updates.subscribe();
        loop {
            let (index, status) = *updates.borrow_and_update();
            if index > 0 && index >= r.nextvalue {
                return Ok(Response::new(pb::WaitResponse {
                    subsystem: 2,
                    updated: Some(index),
                    sendpays: Some(pb::WaitSendpays {
                        status: Some(status),
                        payment_hash: Some(hex::decode(&self.0.hash).unwrap()),
                        partid: Some(1),
                        groupid: Some(1),
                    }),
                    ..Default::default()
                }));
            }
            updates.changed().await.map_err(|_| Status::unavailable("closed"))?;
        }
    }
}
