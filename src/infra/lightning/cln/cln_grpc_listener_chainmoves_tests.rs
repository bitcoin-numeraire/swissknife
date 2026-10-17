//! Adapter component tests, gated by `itest`: a loopback gRPC fixture exercises
//! the actual listener loop. Wallet and application ports use generated mocks.
//! No CLN node, database, or public service is contacted.

use std::{
    convert::Infallible,
    sync::{
        atomic::{AtomicU64, Ordering},
        Arc, Mutex,
    },
    task::{Context, Poll},
    time::Duration,
};

use tonic::{
    codegen::{Body, BoxFuture, Service, StdError},
    server::{NamedService, UnaryService},
    transport::{Channel, Server},
    Request, Response, Status,
};

use super::ClnGrpcListener;
use crate::{
    application::{
        composition::MockAppServicesBuilder,
        errors::{ApplicationError, BitcoinError, DataError, LightningError},
    },
    domains::{
        bitcoin::{BtcOutput, MockBitcoinWallet, OnchainSyncCursor},
        event::OnchainDepositEvent,
    },
    infra::lightning::cln::cln::{
        listchainmoves_chainmoves::ListchainmovesChainmovesPrimaryTag,
        listchainmoves_request::ListchainmovesIndex,
        node_client::NodeClient,
        wait_request::{WaitIndexname, WaitSubsystem},
        ListchainmovesChainmoves, ListchainmovesRequest, ListchainmovesResponse, Outpoint, WaitRequest, WaitResponse,
    },
};

const START: u64 = 41;
const TIP: u64 = 42;
const TXID: [u8; 32] = [1; 32];
const OUTNUM: u32 = 1;

#[derive(Clone, Default)]
struct ChainmovesNode {
    starts: Arc<Mutex<Vec<u64>>>,
}

struct WaitRpc;

impl UnaryService<WaitRequest> for WaitRpc {
    type Response = WaitResponse;
    type Future = BoxFuture<Response<Self::Response>, Status>;

    fn call(&mut self, request: Request<WaitRequest>) -> Self::Future {
        Box::pin(async move {
            let request = request.into_inner();
            assert_eq!(request.subsystem, WaitSubsystem::Chainmoves as i32);
            assert_eq!(request.indexname, WaitIndexname::Created as i32);
            if request.nextvalue > TIP {
                // End a successful batch deterministically rather than hanging
                // forever waiting for another event in this finite fixture.
                return Err(Status::unavailable("end of chainmoves fixture"));
            }
            Ok(Response::new(WaitResponse {
                created: Some(TIP),
                ..Default::default()
            }))
        })
    }
}

impl UnaryService<ListchainmovesRequest> for ChainmovesNode {
    type Response = ListchainmovesResponse;
    type Future = BoxFuture<Response<Self::Response>, Status>;

    fn call(&mut self, request: Request<ListchainmovesRequest>) -> Self::Future {
        let starts = self.starts.clone();
        Box::pin(async move {
            let request = request.into_inner();
            assert_eq!(request.index, Some(ListchainmovesIndex::Created as i32));
            let start = request.start.expect("listener must supply its cursor");
            starts.lock().unwrap().push(start);
            let chainmoves = vec![
                ListchainmovesChainmoves {
                    created_index: START,
                    account_id: "wallet".to_string(),
                    primary_tag: ListchainmovesChainmovesPrimaryTag::Deposit as i32,
                    utxo: Some(Outpoint {
                        txid: TXID.to_vec(),
                        outnum: OUTNUM,
                    }),
                    blockheight: 100,
                    ..Default::default()
                },
                ListchainmovesChainmoves {
                    created_index: TIP,
                    account_id: "external".to_string(),
                    primary_tag: ListchainmovesChainmovesPrimaryTag::ToThem as i32,
                    ..Default::default()
                },
            ]
            .into_iter()
            .filter(|entry| entry.created_index >= start)
            .collect();
            Ok(Response::new(ListchainmovesResponse { chainmoves }))
        })
    }
}

impl<B> Service<http::Request<B>> for ChainmovesNode
where
    B: Body + Send + 'static,
    B::Error: Into<StdError> + Send + 'static,
{
    type Response = http::Response<tonic::body::Body>;
    type Error = Infallible;
    type Future = BoxFuture<Self::Response, Self::Error>;

    fn poll_ready(&mut self, _: &mut Context<'_>) -> Poll<Result<(), Self::Error>> {
        Poll::Ready(Ok(()))
    }

    fn call(&mut self, request: http::Request<B>) -> Self::Future {
        let node = self.clone();
        Box::pin(async move {
            let response = match request.uri().path() {
                "/cln.Node/Wait" => {
                    tonic::server::Grpc::new(tonic_prost::ProstCodec::default())
                        .unary(WaitRpc, request)
                        .await
                }
                "/cln.Node/ListChainMoves" => {
                    tonic::server::Grpc::new(tonic_prost::ProstCodec::default())
                        .unary(node, request)
                        .await
                }
                _ => Status::unimplemented("unexpected fixture RPC").into_http(),
            };
            Ok(response)
        })
    }
}

impl NamedService for ChainmovesNode {
    const NAME: &'static str = "cln.Node";
}

struct Fixture {
    client: NodeClient<Channel>,
    starts: Arc<Mutex<Vec<u64>>>,
    server: tokio::task::JoinHandle<()>,
}

impl Fixture {
    async fn start() -> Self {
        let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
        let address = listener.local_addr().unwrap();
        let node = ChainmovesNode::default();
        let starts = node.starts.clone();
        let incoming = futures_util::stream::unfold(listener, |listener| async {
            Some((listener.accept().await.map(|(stream, _)| stream), listener))
        });
        let server = tokio::spawn(async move {
            Server::builder()
                .add_service(node)
                .serve_with_incoming(incoming)
                .await
                .unwrap();
        });
        let client = NodeClient::connect(format!("http://{address}")).await.unwrap();
        Self { client, starts, server }
    }
}

impl Drop for Fixture {
    fn drop(&mut self) {
        self.server.abort();
    }
}

fn cursor_services() -> (MockAppServicesBuilder, Arc<AtomicU64>) {
    let mut services = MockAppServicesBuilder::new();
    let cursor = Arc::new(AtomicU64::new(START));
    services.system.expect_get_onchain_cursor().returning({
        let cursor = cursor.clone();
        move || Ok(Some(OnchainSyncCursor::CreatedIndex(cursor.load(Ordering::SeqCst))))
    });
    services.system.expect_set_onchain_cursor().returning({
        let cursor = cursor.clone();
        move |next| {
            let OnchainSyncCursor::CreatedIndex(next) = next else {
                panic!("wrong cursor kind")
            };
            cursor.store(next, Ordering::SeqCst);
            Ok(())
        }
    });
    (services, cursor)
}

fn matches_outpoint(txid: &str, index: &Option<u32>, address: &Option<&str>) -> bool {
    txid == hex::encode(TXID) && *index == Some(OUTNUM) && address.is_none()
}

fn output() -> BtcOutput {
    BtcOutput {
        txid: hex::encode(TXID),
        output_index: OUTNUM,
        address: "regtest-deposit-fixture".to_string(),
        amount_sat: 30,
        block_height: Some(100),
        ..Default::default()
    }
}

fn matches_deposit(event: &OnchainDepositEvent) -> bool {
    let expected = output();
    event.txid == expected.txid
        && event.output_index == expected.output_index
        && event.address == expected.address
        && event.amount_sat == expected.amount_sat
        && event.block_height == expected.block_height
}

async fn run_batch(listener: &ClnGrpcListener) -> LightningError {
    tokio::time::timeout(Duration::from_secs(5), listener.listen_chainmoves())
        .await
        .expect("finite fixture must not hang")
        .unwrap_err()
}

async fn replay_after_lookup_failure(include_spent: bool) {
    let fixture = Fixture::start().await;
    let (mut services, cursor) = cursor_services();
    services
        .event
        .expect_onchain_deposit()
        .withf(matches_deposit)
        .times(1)
        .returning(|_| Ok(true));
    let mut wallet = MockBitcoinWallet::new();
    if include_spent {
        wallet
            .expect_get_output()
            .withf(|txid, index, address, spent| matches_outpoint(txid, index, address) && !spent)
            .times(2)
            .returning(|_, _, _, _| Ok(None));
    }
    let mut sequence = mockall::Sequence::new();
    wallet
        .expect_get_output()
        .withf(move |txid, index, address, spent| matches_outpoint(txid, index, address) && *spent == include_spent)
        .times(1)
        .in_sequence(&mut sequence)
        .returning(|_, _, _, _| Err(BitcoinError::GetOutput("temporary wallet lookup failure".into())));
    wallet
        .expect_get_output()
        .withf(move |txid, index, address, spent| matches_outpoint(txid, index, address) && *spent == include_spent)
        .times(1)
        .in_sequence(&mut sequence)
        .returning(|_, _, _, _| Ok(Some(output())));
    let listener = ClnGrpcListener {
        client: fixture.client.clone(),
        services: Arc::new(services.build()),
        wallet: Arc::new(wallet),
    };

    let error = run_batch(&listener).await;
    assert!(error.to_string().contains("temporary wallet lookup failure"), "{error}");
    assert_eq!(cursor.load(Ordering::SeqCst), START);

    // Re-entering the actual listener reloads the persisted cursor, just as the
    // supervisor does after reconnect; the same deposit must be listed again.
    let error = run_batch(&listener).await;
    assert!(error.to_string().contains("end of chainmoves fixture"), "{error}");
    assert_eq!(cursor.load(Ordering::SeqCst), TIP + 1);
    assert_eq!(*fixture.starts.lock().unwrap(), [START, START]);
}

mod listen_chainmoves {
    use super::*;

    mod after_a_transient_unspent_lookup_failure {
        use super::*;

        #[tokio::test]
        async fn replays_the_same_deposit_before_advancing() {
            replay_after_lookup_failure(false).await;
        }
    }

    mod after_a_transient_spent_fallback_failure {
        use super::*;

        #[tokio::test]
        async fn replays_the_same_deposit_before_advancing() {
            replay_after_lookup_failure(true).await;
        }
    }

    mod when_an_unspent_output_is_found {
        use super::*;

        #[tokio::test]
        async fn projects_the_deposit_without_a_spent_query() {
            let fixture = Fixture::start().await;
            let (mut services, cursor) = cursor_services();
            services
                .event
                .expect_onchain_deposit()
                .withf(matches_deposit)
                .times(1)
                .returning(|_| Ok(true));
            let mut wallet = MockBitcoinWallet::new();
            wallet
                .expect_get_output()
                .withf(|txid, index, address, spent| matches_outpoint(txid, index, address) && !spent)
                .times(1)
                .returning(|_, _, _, _| Ok(Some(output())));
            let listener = ClnGrpcListener {
                client: fixture.client.clone(),
                services: Arc::new(services.build()),
                wallet: Arc::new(wallet),
            };

            let error = run_batch(&listener).await;

            assert!(error.to_string().contains("end of chainmoves fixture"), "{error}");
            assert_eq!(cursor.load(Ordering::SeqCst), TIP + 1);
            assert_eq!(*fixture.starts.lock().unwrap(), [START]);
        }
    }

    mod when_the_output_is_missing_from_both_queries {
        use super::*;

        #[tokio::test]
        async fn retains_the_existing_skip_behavior() {
            let fixture = Fixture::start().await;
            let (services, cursor) = cursor_services();
            let mut wallet = MockBitcoinWallet::new();
            let mut sequence = mockall::Sequence::new();
            for include_spent in [false, true] {
                wallet
                    .expect_get_output()
                    .withf(move |txid, index, address, spent| {
                        matches_outpoint(txid, index, address) && *spent == include_spent
                    })
                    .times(1)
                    .in_sequence(&mut sequence)
                    .returning(|_, _, _, _| Ok(None));
            }
            let listener = ClnGrpcListener {
                client: fixture.client.clone(),
                services: Arc::new(services.build()),
                wallet: Arc::new(wallet),
            };

            let error = run_batch(&listener).await;

            assert!(error.to_string().contains("end of chainmoves fixture"), "{error}");
            assert_eq!(cursor.load(Ordering::SeqCst), TIP + 1);
        }
    }

    mod when_deposit_projection_fails {
        use super::*;

        #[tokio::test]
        async fn preserves_the_cursor() {
            let fixture = Fixture::start().await;
            let (mut services, cursor) = cursor_services();
            services
                .event
                .expect_onchain_deposit()
                .withf(matches_deposit)
                .times(1)
                .returning(|_| {
                    Err(ApplicationError::Data(DataError::Inconsistency(
                        "temporary deposit projection failure".into(),
                    )))
                });
            let mut wallet = MockBitcoinWallet::new();
            wallet
                .expect_get_output()
                .withf(|txid, index, address, spent| matches_outpoint(txid, index, address) && !spent)
                .times(1)
                .returning(|_, _, _, _| Ok(Some(output())));
            let listener = ClnGrpcListener {
                client: fixture.client.clone(),
                services: Arc::new(services.build()),
                wallet: Arc::new(wallet),
            };

            let error = run_batch(&listener).await;

            assert!(matches!(error, LightningError::EventProcessing(_)), "{error}");
            assert_eq!(cursor.load(Ordering::SeqCst), START);
        }
    }

    mod when_the_unspent_lookup_fails {
        use super::*;

        #[tokio::test]
        async fn preserves_the_cursor_for_replay() {
            let fixture = Fixture::start().await;
            let (services, cursor) = cursor_services();
            let mut wallet = MockBitcoinWallet::new();
            wallet
                .expect_get_output()
                .withf(|txid, index, address, spent| matches_outpoint(txid, index, address) && !spent)
                .times(1)
                .returning(|_, _, _, _| Err(BitcoinError::GetOutput("temporary wallet lookup failure".into())));
            let listener = ClnGrpcListener {
                client: fixture.client.clone(),
                services: Arc::new(services.build()),
                wallet: Arc::new(wallet),
            };

            let error = tokio::time::timeout(Duration::from_secs(5), listener.listen_chainmoves())
                .await
                .expect("listener must return on lookup failure")
                .unwrap_err();

            assert_eq!(
                cursor.load(Ordering::SeqCst),
                START,
                "failed lookup must not checkpoint past the deposit"
            );
            assert!(error.to_string().contains("temporary wallet lookup failure"), "{error}");
            assert_eq!(*fixture.starts.lock().unwrap(), [START]);
        }
    }
}
