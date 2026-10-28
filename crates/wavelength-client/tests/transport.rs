//! Transport-contract tests, separate from the real-daemon Docker tests.
use std::{
    pin::Pin,
    sync::{
        atomic::{AtomicUsize, Ordering},
        Arc,
    },
    time::Duration,
};
use tokio::net::TcpListener;
use tokio_stream::{
    wrappers::{ReceiverStream, TcpListenerStream},
    Stream,
};
use tonic::{
    transport::{Identity, Server, ServerTlsConfig},
    Request, Response, Status,
};
use wavelength_client::{
    rpc::{
        self,
        wallet_service_server::{WalletService, WalletServiceServer},
    },
    Client, Config, Error,
};

const CERT: &[u8] = include_bytes!("fixtures/cert.pem");
const KEY: &[u8] = include_bytes!("fixtures/key.pem");
const TOKEN: &[u8] = b"test credential";

#[derive(Clone, Default)]
struct Wallet {
    mode: u8,
    sends: Arc<AtomicUsize>,
}

#[tonic::async_trait]
impl WalletService for Wallet {
    async fn create(&self, _request: Request<rpc::CreateRequest>) -> Result<Response<rpc::CreateResponse>, Status> {
        Err(Status::unimplemented("outside test contract"))
    }
    async fn unlock(&self, _request: Request<rpc::UnlockRequest>) -> Result<Response<rpc::UnlockResponse>, Status> {
        Err(Status::unimplemented("outside test contract"))
    }
    async fn prepare_send(
        &self,
        _request: Request<rpc::PrepareSendRequest>,
    ) -> Result<Response<rpc::PrepareSendResponse>, Status> {
        Err(Status::unimplemented("outside test contract"))
    }
    async fn send(&self, _request: Request<rpc::SendRequest>) -> Result<Response<rpc::SendResponse>, Status> {
        self.sends.fetch_add(1, Ordering::SeqCst);
        Err(Status::unavailable("connection lost after dispatch"))
    }
    async fn recv(&self, request: Request<rpc::RecvRequest>) -> Result<Response<rpc::RecvResponse>, Status> {
        Ok(Response::new(rpc::RecvResponse {
            invoice: request.into_inner().memo,
            ..Default::default()
        }))
    }
    async fn list(&self, request: Request<rpc::ListRequest>) -> Result<Response<rpc::ListResponse>, Status> {
        let input = request.into_inner();
        assert_eq!(input.offset, 0);
        assert_eq!(input.limit, 1);
        let (id, has_more, next_cursor) = match input.cursor.as_str() {
            "" => ("credit-operation", true, if self.mode == 2 { "" } else { "second" }),
            "second" => ("other-entry", self.mode == 3, "second"),
            _ => return Err(Status::invalid_argument("bad cursor")),
        };
        Ok(Response::new(rpc::ListResponse {
            body: Some(rpc::list_response::Body::Activity(rpc::ActivityList {
                entries: vec![rpc::WalletEntry {
                    id: id.into(),
                    progress: Some(rpc::WalletEntryProgress {
                        payment_hash: "payment-hash".into(),
                        ..Default::default()
                    }),
                    ..Default::default()
                }],
                has_more,
                next_cursor: next_cursor.into(),
                total: 2,
            })),
        }))
    }
    async fn deposit(&self, _request: Request<rpc::DepositRequest>) -> Result<Response<rpc::DepositResponse>, Status> {
        Err(Status::unimplemented("outside test contract"))
    }
    async fn balance(&self, _request: Request<rpc::BalanceRequest>) -> Result<Response<rpc::BalanceResponse>, Status> {
        Err(Status::unimplemented("outside test contract"))
    }
    async fn status(&self, _request: Request<rpc::StatusRequest>) -> Result<Response<rpc::StatusResponse>, Status> {
        if self.mode == 1 {
            tokio::time::sleep(Duration::from_secs(2)).await;
        }
        Ok(Response::new(rpc::StatusResponse {
            ready: true,
            unlocked: true,
            network: "regtest".into(),
            ..Default::default()
        }))
    }
    async fn get_exit_plan(
        &self,
        _request: Request<rpc::GetExitPlanRequest>,
    ) -> Result<Response<rpc::GetExitPlanResponse>, Status> {
        Err(Status::unimplemented("outside test contract"))
    }
    async fn sweep_wallet(
        &self,
        _request: Request<rpc::SweepWalletRequest>,
    ) -> Result<Response<rpc::SweepWalletResponse>, Status> {
        Err(Status::unimplemented("outside test contract"))
    }
    async fn exit(&self, _request: Request<rpc::ExitRequest>) -> Result<Response<rpc::ExitResponse>, Status> {
        Err(Status::unimplemented("outside test contract"))
    }
    async fn exit_status(
        &self,
        _request: Request<rpc::ExitStatusRequest>,
    ) -> Result<Response<rpc::ExitStatusResponse>, Status> {
        Err(Status::unimplemented("outside test contract"))
    }
    async fn exit_summary(
        &self,
        _request: Request<rpc::ExitSummaryRequest>,
    ) -> Result<Response<rpc::ExitSummaryResponse>, Status> {
        Err(Status::unimplemented("outside test contract"))
    }
    type SubscribeWalletStream = Pin<Box<dyn Stream<Item = Result<rpc::SubscribeWalletResponse, Status>> + Send>>;
    async fn subscribe_wallet(
        &self,
        _request: Request<rpc::SubscribeWalletRequest>,
    ) -> Result<Response<Self::SubscribeWalletStream>, Status> {
        let (tx, rx) = tokio::sync::mpsc::channel(2);
        tokio::spawn(async move {
            let _ = tx
                .send(Ok(rpc::SubscribeWalletResponse {
                    cursor: 1,
                    ..Default::default()
                }))
                .await;
            tokio::time::sleep(Duration::from_millis(300)).await;
            let _ = tx
                .send(Ok(rpc::SubscribeWalletResponse {
                    cursor: 3,
                    ..Default::default()
                }))
                .await;
        });
        Ok(Response::new(Box::pin(ReceiverStream::new(rx))))
    }
}

struct TestServer {
    endpoint: String,
    task: tokio::task::JoinHandle<()>,
    sends: Arc<AtomicUsize>,
}
impl Drop for TestServer {
    fn drop(&mut self) {
        self.task.abort();
    }
}
impl TestServer {
    async fn start(mode: u8) -> Self {
        let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
        let endpoint = format!("https://localhost:{}", listener.local_addr().unwrap().port());
        let wallet = Wallet {
            mode,
            ..Default::default()
        };
        let sends = wallet.sends.clone();
        let service = WalletServiceServer::with_interceptor(wallet, |request: Request<()>| {
            if request.metadata().get("macaroon").and_then(|v| v.to_str().ok()) != Some(hex::encode(TOKEN).as_str()) {
                return Err(Status::unauthenticated("missing or invalid credential"));
            }
            Ok(request)
        });
        let task = tokio::spawn(async move {
            Server::builder()
                .tls_config(ServerTlsConfig::new().identity(Identity::from_pem(CERT, KEY)))
                .unwrap()
                .add_service(service)
                .serve_with_incoming(TcpListenerStream::new(listener))
                .await
                .unwrap();
        });
        Self { endpoint, task, sends }
    }
    fn config(&self) -> Config {
        Config {
            endpoint: self.endpoint.clone(),
            certificate_pem: CERT.to_vec(),
            macaroon: TOKEN.to_vec(),
            connect_timeout: Duration::from_secs(2),
            request_timeout: Duration::from_millis(150),
        }
    }
    async fn client(&self) -> Client {
        Client::connect(self.config()).await.unwrap()
    }
}

#[tokio::test]
async fn verifies_tls_and_attaches_binary_macaroon_as_hex() {
    let server = TestServer::start(0).await;
    let client = server.client().await;
    assert!(client.status(Default::default()).await.unwrap().ready);
    let response = client
        .recv(rpc::RecvRequest {
            amt_sat: 21,
            memo: "memo preserved".into(),
        })
        .await
        .unwrap();
    assert_eq!(response.invoice, "memo preserved");
    assert!(!format!("{client:?}").contains(&hex::encode(TOKEN)));
}

#[tokio::test]
async fn rejects_bad_authentication() {
    let server = TestServer::start(0).await;
    let mut config = server.config();
    config.macaroon = b"wrong".to_vec();
    let client = Client::connect(config).await.unwrap();
    assert!(
        matches!(client.status(Default::default()).await, Err(Error::Rpc(status)) if status.code() == tonic::Code::Unauthenticated)
    );
}

#[tokio::test]
async fn rejects_untrusted_tls_and_plaintext() {
    let server = TestServer::start(0).await;
    let mut config = server.config();
    config.certificate_pem = b"not a certificate".to_vec();
    assert!(Client::connect(config).await.is_err());
    let mut config = server.config();
    config.endpoint = config.endpoint.replace("https:", "http:");
    assert!(matches!(Client::connect(config).await, Err(Error::Config(_))));
}

#[tokio::test]
async fn bounds_unary_calls_without_retrying_a_submitted_send() {
    let server = TestServer::start(1).await;
    let client = server.client().await;
    let started = std::time::Instant::now();
    assert!(client.status(Default::default()).await.is_err());
    assert!(started.elapsed() < Duration::from_secs(1));
    assert!(client
        .send(rpc::SendRequest {
            send_intent_id: "intent".into()
        })
        .await
        .is_err());
    assert_eq!(server.sends.load(Ordering::SeqCst), 1);
}

#[tokio::test]
async fn does_not_apply_unary_deadlines_to_live_streams() {
    let server = TestServer::start(0).await;
    let mut stream = server
        .client()
        .await
        .subscribe_wallet(Default::default())
        .await
        .unwrap();
    assert_eq!(stream.message().await.unwrap().unwrap().cursor, 1);
    assert_eq!(stream.message().await.unwrap().unwrap().cursor, 3);
}

#[tokio::test]
async fn reads_every_activity_page_and_preserves_distinct_receive_ids() {
    let server = TestServer::start(0).await;
    let entries = server.client().await.activity(1, 2).await.unwrap();
    assert_eq!(entries.len(), 2);
    assert_eq!(entries[0].id, "credit-operation");
    assert_eq!(entries[0].progress.as_ref().unwrap().payment_hash, "payment-hash");
}

#[tokio::test]
async fn rejects_missing_repeated_and_unbounded_activity_cursors() {
    for mode in [2, 3] {
        let server = TestServer::start(mode).await;
        assert!(matches!(
            server.client().await.activity(1, 4).await,
            Err(Error::Pagination(_))
        ));
    }
    let server = TestServer::start(0).await;
    assert!(matches!(
        server.client().await.activity(1, 1).await,
        Err(Error::Pagination(_))
    ));
    assert!(matches!(
        server.client().await.activity(0, 1).await,
        Err(Error::Pagination(_))
    ));
}

#[test]
fn converts_only_exact_satoshi_amounts() {
    assert_eq!(wavelength_client::exact_sats(21_000), Some(21));
    assert_eq!(wavelength_client::exact_sats(21_001), None);
    assert_eq!(wavelength_client::exact_sats(u64::MAX), None);
}
