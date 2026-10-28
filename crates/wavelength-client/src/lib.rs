//! Typed, authenticated access to an external Wavelength wallet daemon.
//!
//! This crate owns transport and wire types only. Wallet creation, key custody,
//! accounting, daemon supervision and deciding whether to retry a money-moving
//! RPC belong to the caller. In particular, a failed `send` response does not
//! establish that a send failed: reconcile durable activity before doing anything
//! that could dispatch it again.

use std::{collections::HashSet, fmt, time::Duration};

use tonic::{
    metadata::AsciiMetadataValue,
    service::{interceptor::InterceptedService, Interceptor},
    transport::{Certificate, Channel, ClientTlsConfig, Endpoint},
    Request, Status, Streaming,
};

/// Wire contract pinned to the revision documented in `proto/README.md`.
#[allow(clippy::all)]
pub mod rpc {
    tonic::include_proto!("wavewalletrpc");
}

/// Public, daemon-owned swap read APIs, pinned alongside the wallet contract.
#[allow(clippy::all)]
pub mod swaps {
    tonic::include_proto!("swapclientrpc");
}

/// Configuration contains credential bytes and intentionally does not implement
/// `Debug`. Supply the daemon CA/leaf certificate and the **binary** macaroon.
pub struct Config {
    pub endpoint: String,
    pub certificate_pem: Vec<u8>,
    pub macaroon: Vec<u8>,
    pub connect_timeout: Duration,
    pub request_timeout: Duration,
}

#[derive(Debug, thiserror::Error)]
pub enum Error {
    #[error("Invalid Wavelength client configuration: {0}")]
    Config(&'static str),
    #[error("Wavelength transport error: {0}")]
    Transport(#[from] tonic::transport::Error),
    #[error("Wavelength RPC error: {0}")]
    Rpc(#[from] Status),
    #[error("Wavelength RPC timed out; a submitted operation may still be running")]
    Timeout,
    #[error("Invalid Wavelength activity pagination: {0}")]
    Pagination(&'static str),
}

#[derive(Clone)]
struct Macaroon(AsciiMetadataValue);

impl Interceptor for Macaroon {
    fn call(&mut self, mut request: Request<()>) -> Result<Request<()>, Status> {
        request.metadata_mut().insert("macaroon", self.0.clone());
        Ok(request)
    }
}

type AuthenticatedChannel = InterceptedService<Channel, Macaroon>;

/// Clones share a multiplexed HTTP/2 channel. RPC methods do not automatically
/// retry, including after a deadline or connection loss.
#[derive(Clone)]
pub struct Client {
    wallet: rpc::wallet_service_client::WalletServiceClient<AuthenticatedChannel>,
    inspection: rpc::wallet_inspection_service_client::WalletInspectionServiceClient<AuthenticatedChannel>,
    swaps: swaps::swap_client_service_client::SwapClientServiceClient<AuthenticatedChannel>,
    request_timeout: Duration,
}

impl fmt::Debug for Client {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("WavelengthClient").finish_non_exhaustive()
    }
}

macro_rules! unary {
    ($($method:ident($request:ident) -> $response:ident),+ $(,)?) => {
        $(pub async fn $method(&self, input: rpc::$request) -> Result<rpc::$response, Error> {
            let mut request = Request::new(input);
            request.set_timeout(self.request_timeout);
            let response = tokio::time::timeout(
                self.request_timeout,
                self.wallet.clone().$method(request),
            ).await.map_err(|_| Error::Timeout)??;
            Ok(response.into_inner())
        })+
    };
}

impl Client {
    pub async fn connect(config: Config) -> Result<Self, Error> {
        if config.connect_timeout.is_zero() || config.request_timeout.is_zero() {
            return Err(Error::Config("timeouts must be positive"));
        }
        let endpoint = Endpoint::from_shared(config.endpoint).map_err(|_| Error::Config("invalid endpoint"))?;
        if endpoint.uri().scheme_str() != Some("https") {
            return Err(Error::Config("HTTPS is required"));
        }
        if config.certificate_pem.is_empty() || config.macaroon.is_empty() {
            return Err(Error::Config("certificate and macaroon are required"));
        }
        let macaroon: AsciiMetadataValue = hex::encode(config.macaroon)
            .parse()
            .map_err(|_| Error::Config("invalid macaroon"))?;
        let channel = endpoint
            .connect_timeout(config.connect_timeout)
            .tls_config(ClientTlsConfig::new().ca_certificate(Certificate::from_pem(config.certificate_pem)))?
            .connect()
            .await?;
        Ok(Self {
            wallet: rpc::wallet_service_client::WalletServiceClient::with_interceptor(
                channel.clone(),
                Macaroon(macaroon.clone()),
            ),
            inspection: rpc::wallet_inspection_service_client::WalletInspectionServiceClient::with_interceptor(
                channel.clone(),
                Macaroon(macaroon.clone()),
            ),
            swaps: swaps::swap_client_service_client::SwapClientServiceClient::with_interceptor(
                channel,
                Macaroon(macaroon),
            ),
            request_timeout: config.request_timeout,
        })
    }

    /// Read-only quote: unlike PrepareSend, this allocates no intent or credit
    /// earmark. No RPC is retried automatically.
    pub async fn quote_pay(&self, input: swaps::QuotePayRequest) -> Result<swaps::QuotePayResponse, Error> {
        let mut request = Request::new(input);
        request.set_timeout(self.request_timeout);
        Ok(
            tokio::time::timeout(self.request_timeout, self.swaps.clone().quote_pay(request))
                .await
                .map_err(|_| Error::Timeout)??
                .into_inner(),
        )
    }

    /// Fetch one persisted swap by its stable payment hash, independent of the
    /// inspection service's capped recent-activity window.
    pub async fn get_swap(&self, input: swaps::GetSwapRequest) -> Result<swaps::GetSwapResponse, Error> {
        let mut request = Request::new(input);
        request.set_timeout(self.request_timeout);
        Ok(
            tokio::time::timeout(self.request_timeout, self.swaps.clone().get_swap(request))
                .await
                .map_err(|_| Error::Timeout)??
                .into_inner(),
        )
    }

    /// Read the server-authoritative credit snapshot. The public API has a
    /// limit but no cursor: an absent operation is never proof of failure.
    pub async fn list_credits(&self, input: swaps::ListCreditsRequest) -> Result<swaps::ListCreditsResponse, Error> {
        let mut request = Request::new(input);
        request.set_timeout(self.request_timeout);
        Ok(
            tokio::time::timeout(self.request_timeout, self.swaps.clone().list_credits(request))
                .await
                .map_err(|_| Error::Timeout)??
                .into_inner(),
        )
    }

    unary! {
        create(CreateRequest) -> CreateResponse,
        unlock(UnlockRequest) -> UnlockResponse,
        status(StatusRequest) -> StatusResponse,
        balance(BalanceRequest) -> BalanceResponse,
        recv(RecvRequest) -> RecvResponse,
        prepare_send(PrepareSendRequest) -> PrepareSendResponse,
        send(SendRequest) -> SendResponse,
        list(ListRequest) -> ListResponse,
        deposit(DepositRequest) -> DepositResponse,
        get_exit_plan(GetExitPlanRequest) -> GetExitPlanResponse,
        sweep_wallet(SweepWalletRequest) -> SweepWalletResponse,
        exit(ExitRequest) -> ExitResponse,
        exit_status(ExitStatusRequest) -> ExitStatusResponse,
        exit_summary(ExitSummaryRequest) -> ExitSummaryResponse,
    }

    pub async fn inspect_activity(
        &self,
        input: rpc::InspectActivityRequest,
    ) -> Result<rpc::InspectActivityResponse, Error> {
        let mut request = Request::new(input);
        request.set_timeout(self.request_timeout);
        let response = tokio::time::timeout(self.request_timeout, self.inspection.clone().inspect_activity(request))
            .await
            .map_err(|_| Error::Timeout)??;
        Ok(response.into_inner())
    }

    /// The deadline bounds opening the stream, not its lifetime. A consumer must
    /// reconnect and replay persisted activity when a stream ends or fails.
    pub async fn subscribe_wallet(
        &self,
        input: rpc::SubscribeWalletRequest,
    ) -> Result<Streaming<rpc::SubscribeWalletResponse>, Error> {
        let response = tokio::time::timeout(self.request_timeout, self.wallet.clone().subscribe_wallet(input))
            .await
            .map_err(|_| Error::Timeout)??;
        Ok(response.into_inner())
    }

    /// Read the complete activity snapshot, with explicit bounds. Incomplete or
    /// malformed pagination is an error, never evidence that a payment is absent.
    /// Credit receive IDs differ from payment hashes: inspect the structured
    /// request/progress fields when correlating payments.
    pub async fn activity(&self, page_size: u32, max_pages: usize) -> Result<Vec<rpc::WalletEntry>, Error> {
        if page_size == 0 || max_pages == 0 {
            return Err(Error::Pagination("page size and maximum pages must be positive"));
        }
        let mut cursor = String::new();
        let mut cursors = HashSet::new();
        let mut entries = Vec::new();
        for _ in 0..max_pages {
            let response = self
                .list(rpc::ListRequest {
                    view: rpc::ListView::Activity.into(),
                    limit: page_size,
                    cursor: cursor.clone(),
                    ..Default::default()
                })
                .await?;
            let Some(rpc::list_response::Body::Activity(page)) = response.body else {
                return Err(Error::Pagination("expected activity response"));
            };
            entries.extend(page.entries);
            if !page.has_more {
                return Ok(entries);
            }
            if page.next_cursor.is_empty() || !cursors.insert(page.next_cursor.clone()) {
                return Err(Error::Pagination("missing or repeated cursor"));
            }
            cursor = page.next_cursor;
        }
        Err(Error::Pagination("activity page limit exceeded"))
    }
}

/// Convert an exact millisatoshi amount to the daemon's satoshi unit. Never
/// round user amounts or fee limits to fit the transport.
pub fn exact_sats(amount_msat: u64) -> Option<u64> {
    amount_msat.is_multiple_of(1000).then_some(amount_msat / 1000)
}
