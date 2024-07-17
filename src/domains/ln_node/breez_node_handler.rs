use std::sync::Arc;

use axum::{
    body::Body,
    extract::State,
    http::header,
    response::{IntoResponse, Response},
    routing::{get, post},
    Json, Router,
};
use breez_sdk_core::{LspInformation, NodeState, ReverseSwapInfo};
use utoipa::OpenApi;

use crate::{
    application::{
        docs::{
            BAD_REQUEST_EXAMPLE, FORBIDDEN_EXAMPLE, INTERNAL_EXAMPLE, NOT_FOUND_EXAMPLE,
            UNAUTHORIZED_EXAMPLE,
        },
        dtos::{
            CheckMessageRequest, CheckMessageResponse, ConnectLSPRequest, RedeemOnchainRequest,
            RedeemOnchainResponse, SendOnchainPaymentRequest, SignMessageRequest,
            SignMessageResponse,
        },
        errors::{ApplicationError, LightningError},
    },
    domains::user::{Permission, User},
    infra::{app::AppState, lightning::LnClient},
};

#[derive(OpenApi)]
#[openapi(
    paths(node_info, lsp_info, list_lsps, close_lsp_channels, connect_lsp, swap, redeem, sign_message, check_message, sync, backup),
    components(schemas(ConnectLSPRequest, SendOnchainPaymentRequest, RedeemOnchainRequest, RedeemOnchainResponse, SignMessageRequest, CheckMessageRequest, SignMessageResponse, CheckMessageResponse)),
    tags(
        (name = "Lightning Node", description = "LN Node management endpoints. Currently only available for `breez` Lightning provider. Require `read:ln_node` or `write:ln_node` permissions.")
    )
)]
pub struct BreezNodeHandler;
pub const CONTEXT_PATH: &str = "/api/lightning/node";

pub fn breez_node_router() -> Router<Arc<AppState>> {
    Router::new()
        .route("/info", get(node_info))
        .route("/lsp-info", get(lsp_info))
        .route("/lsps", get(list_lsps))
        .route("/close-channels", post(close_lsp_channels))
        .route("/connect-lsp", post(connect_lsp))
        .route("/swap", post(swap))
        .route("/redeem", post(redeem))
        .route("/sign-message", post(sign_message))
        .route("/check-message", post(check_message))
        .route("/sync", post(sync))
        .route("/backup", get(backup))
}

/// Get node info
///
/// Returns the Core Lightning node info hosted on [Greenlight (Blockstream)](https://blockstream.com/lightning/greenlight/) infrastructure
#[utoipa::path(
    get,
    path = "/info",
    tag = "Lightning Node",
    context_path = CONTEXT_PATH,
    responses(
        (status = 200, description = "Found", body = Value),
        (status = 401, description = "Unauthorized", body = ErrorResponse, example = json!(UNAUTHORIZED_EXAMPLE)),
        (status = 403, description = "Forbidden", body = ErrorResponse, example = json!(FORBIDDEN_EXAMPLE)),
        (status = 500, description = "Internal Server Error", body = ErrorResponse, example = json!(INTERNAL_EXAMPLE))
    )
)]
async fn node_info(
    State(app_state): State<Arc<AppState>>,
    user: User,
) -> Result<Json<NodeState>, ApplicationError> {
    user.check_permission(Permission::ReadLnNode)?;

    let client = app_state.ln_node_client.as_breez_client()?;
    let node_info = client.node_info()?;

    Ok(node_info.into())
}

/// Get LSP info
///
/// Returns the info of the current Breez partner LSP connected to the Core Lightning node.
#[utoipa::path(
    get,
    path = "/lsp-info",
    tag = "Lightning Node",
    context_path = CONTEXT_PATH,
    responses(
        (status = 200, description = "Found", body = Value),
        (status = 401, description = "Unauthorized", body = ErrorResponse, example = json!(UNAUTHORIZED_EXAMPLE)),
        (status = 403, description = "Forbidden", body = ErrorResponse, example = json!(FORBIDDEN_EXAMPLE)),
        (status = 500, description = "Internal Server Error", body = ErrorResponse, example = json!(INTERNAL_EXAMPLE))
    )
)]
async fn lsp_info(
    State(app_state): State<Arc<AppState>>,
    user: User,
) -> Result<Json<LspInformation>, ApplicationError> {
    user.check_permission(Permission::ReadLnNode)?;

    let client = app_state.ln_node_client.as_breez_client()?;
    let lsp_info = client.lsp_info().await?;

    Ok(lsp_info.into())
}

/// List LSPs
///
/// Returns the list of available LSPs for the node.
#[utoipa::path(
    get,
    path = "/lsps",
    tag = "Lightning Node",
    context_path = CONTEXT_PATH,
    responses(
        (status = 200, description = "Success", body = Vec<Value>),
        (status = 401, description = "Unauthorized", body = ErrorResponse, example = json!(UNAUTHORIZED_EXAMPLE)),
        (status = 403, description = "Forbidden", body = ErrorResponse, example = json!(FORBIDDEN_EXAMPLE)),
        (status = 500, description = "Internal Server Error", body = ErrorResponse, example = json!(INTERNAL_EXAMPLE))
    )
)]
async fn list_lsps(
    State(app_state): State<Arc<AppState>>,
    user: User,
) -> Result<Json<Vec<LspInformation>>, ApplicationError> {
    user.check_permission(Permission::ReadLnNode)?;

    let client = app_state.ln_node_client.as_breez_client()?;
    let lsps = client.list_lsps().await?;

    Ok(lsps.into())
}

/// Close LSP channels
///
/// Returns the list of transaction IDs for the lightning channel closures. The funds are deposited in your on-chain addresses and can be redeemed
#[utoipa::path(
    post,
    path = "/close-channels",
    tag = "Lightning Node",
    context_path = CONTEXT_PATH,
    responses(
        (status = 200, description = "Channels Closed", body = Vec<String>),
        (status = 401, description = "Unauthorized", body = ErrorResponse, example = json!(UNAUTHORIZED_EXAMPLE)),
        (status = 403, description = "Forbidden", body = ErrorResponse, example = json!(FORBIDDEN_EXAMPLE)),
        (status = 500, description = "Internal Server Error", body = ErrorResponse, example = json!(INTERNAL_EXAMPLE))
    )
)]
async fn close_lsp_channels(
    State(app_state): State<Arc<AppState>>,
    user: User,
) -> Result<Json<Vec<String>>, ApplicationError> {
    user.check_permission(Permission::WriteLnNode)?;

    let client = app_state.ln_node_client.as_breez_client()?;
    let tx_ids = client.close_lsp_channels().await?;

    Ok(tx_ids.into())
}

/// Connect LSP
///
/// Connects to an LSP from the list of available LSPs by its ID. Returns an  empty body
#[utoipa::path(
    post,
    path = "/connect-lsp",
    tag = "Lightning Node",
    context_path = CONTEXT_PATH,
    request_body = ConnectLSPRequest,
    responses(
        (status = 200, description = "LSP Connected"),
        (status = 400, description = "Bad Request", body = ErrorResponse, example = json!(BAD_REQUEST_EXAMPLE)),
        (status = 401, description = "Unauthorized", body = ErrorResponse, example = json!(UNAUTHORIZED_EXAMPLE)),
        (status = 403, description = "Forbidden", body = ErrorResponse, example = json!(FORBIDDEN_EXAMPLE)),
        (status = 404, description = "Not Found", body = ErrorResponse, example = json!(NOT_FOUND_EXAMPLE)),
        (status = 500, description = "Internal Server Error", body = ErrorResponse, example = json!(INTERNAL_EXAMPLE))
    )
)]
async fn connect_lsp(
    State(app_state): State<Arc<AppState>>,
    user: User,
    Json(payload): Json<ConnectLSPRequest>,
) -> Result<(), ApplicationError> {
    user.check_permission(Permission::WriteLnNode)?;

    let client = app_state.ln_node_client.as_breez_client()?;
    client.connect_lsp(payload.lsp_id).await?;

    Ok(())
}

/// Swap BTC
///
/// Pays BTC on-chain via Swap service. Meaning that the funds are sent through Lightning and swaps to the recipient on-chain address
#[utoipa::path(
    post,
    path = "/swap",
    tag = "Lightning Node",
    context_path = CONTEXT_PATH,
    request_body = SendOnchainPaymentRequest,
    responses(
        (status = 200, description = "Swap Success", body = Value),
        (status = 400, description = "Bad Request", body = ErrorResponse, example = json!(BAD_REQUEST_EXAMPLE)),
        (status = 401, description = "Unauthorized", body = ErrorResponse, example = json!(UNAUTHORIZED_EXAMPLE)),
        (status = 403, description = "Forbidden", body = ErrorResponse, example = json!(FORBIDDEN_EXAMPLE)),
        (status = 500, description = "Internal Server Error", body = ErrorResponse, example = json!(INTERNAL_EXAMPLE))
    )
)]
async fn swap(
    State(app_state): State<Arc<AppState>>,
    user: User,
    Json(payload): Json<SendOnchainPaymentRequest>,
) -> Result<Json<ReverseSwapInfo>, ApplicationError> {
    user.check_permission(Permission::WriteLnNode)?;

    let client = app_state.ln_node_client.as_breez_client()?;
    let payment_info = client
        .pay_onchain(
            payload.amount_msat,
            payload.recipient_address,
            payload.feerate,
        )
        .await?;

    Ok(payment_info.into())
}

/// Redeem BTC
///
/// Redeems your whole on-chain BTC balance to an address of your choice. Returns the transaction ID.
#[utoipa::path(
    post,
    path = "/redeem",
    tag = "Lightning Node",
    context_path = CONTEXT_PATH,
    request_body = RedeemOnchainRequest,
    responses(
        (status = 200, description = "Redeem Success", body = RedeemOnchainResponse),
        (status = 400, description = "Bad Request", body = ErrorResponse, example = json!(BAD_REQUEST_EXAMPLE)),
        (status = 401, description = "Unauthorized", body = ErrorResponse, example = json!(UNAUTHORIZED_EXAMPLE)),
        (status = 403, description = "Forbidden", body = ErrorResponse, example = json!(FORBIDDEN_EXAMPLE)),
        (status = 500, description = "Internal Server Error", body = ErrorResponse, example = json!(INTERNAL_EXAMPLE))
    )
)]
async fn redeem(
    State(app_state): State<Arc<AppState>>,
    user: User,
    Json(payload): Json<RedeemOnchainRequest>,
) -> Result<Json<RedeemOnchainResponse>, ApplicationError> {
    user.check_permission(Permission::WriteLnNode)?;

    let client = app_state.ln_node_client.as_breez_client()?;
    let txid = client
        .redeem_onchain(payload.to_address, payload.feerate)
        .await?;

    Ok(RedeemOnchainResponse { txid }.into())
}

/// Sign message
///
/// Signs a message using the node's key. Returns a zbase encoded signature
#[utoipa::path(
    post,
    path = "/sign-message",
    tag = "Lightning Node",
    context_path = CONTEXT_PATH,
    request_body = SignMessageRequest,
    responses(
        (status = 200, description = "Message Signed", body = SignMessageResponse),
        (status = 400, description = "Bad Request", body = ErrorResponse, example = json!(BAD_REQUEST_EXAMPLE)),
        (status = 401, description = "Unauthorized", body = ErrorResponse, example = json!(UNAUTHORIZED_EXAMPLE)),
        (status = 403, description = "Forbidden", body = ErrorResponse, example = json!(FORBIDDEN_EXAMPLE)),
        (status = 500, description = "Internal Server Error", body = ErrorResponse, example = json!(INTERNAL_EXAMPLE))
    )
)]
async fn sign_message(
    State(app_state): State<Arc<AppState>>,
    user: User,
    Json(payload): Json<SignMessageRequest>,
) -> Result<Json<SignMessageResponse>, ApplicationError> {
    user.check_permission(Permission::WriteLnNode)?;

    let client = app_state.ln_node_client.as_breez_client()?;
    let signature = client.sign_message(payload.message).await?;

    Ok(SignMessageResponse { signature }.into())
}

/// Verify Signature
///
/// Verifies the validity of a signature against a node's public key. Returns `true` if valid.
#[utoipa::path(
    post,
    path = "/check-message",
    tag = "Lightning Node",
    context_path = CONTEXT_PATH,
    request_body = CheckMessageRequest,
    responses(
        (status = 200, description = "Message Verified", body = CheckMessageResponse),
        (status = 400, description = "Bad Request", body = ErrorResponse, example = json!(BAD_REQUEST_EXAMPLE)),
        (status = 401, description = "Unauthorized", body = ErrorResponse, example = json!(UNAUTHORIZED_EXAMPLE)),
        (status = 403, description = "Forbidden", body = ErrorResponse, example = json!(FORBIDDEN_EXAMPLE)),
        (status = 500, description = "Internal Server Error", body = ErrorResponse, example = json!(INTERNAL_EXAMPLE))
    )
)]
async fn check_message(
    State(app_state): State<Arc<AppState>>,
    user: User,
    Json(payload): Json<CheckMessageRequest>,
) -> Result<Json<CheckMessageResponse>, ApplicationError> {
    user.check_permission(Permission::WriteLnNode)?;

    let client = app_state.ln_node_client.as_breez_client()?;
    let is_valid = client
        .check_message(payload.message, payload.pubkey, payload.signature)
        .await?;

    Ok(CheckMessageResponse { is_valid }.into())
}

/// Sync node
///
/// Syncs the local state with the remote node state.
#[utoipa::path(
    post,
    path = "/sync",
    tag = "Lightning Node",
    context_path = CONTEXT_PATH,
    responses(
        (status = 200, description = "Node Synced"),
        (status = 401, description = "Unauthorized", body = ErrorResponse, example = json!(UNAUTHORIZED_EXAMPLE)),
        (status = 403, description = "Forbidden", body = ErrorResponse, example = json!(FORBIDDEN_EXAMPLE)),
        (status = 500, description = "Internal Server Error", body = ErrorResponse, example = json!(INTERNAL_EXAMPLE))
    )
)]
async fn sync(State(app_state): State<Arc<AppState>>, user: User) -> Result<(), ApplicationError> {
    user.check_permission(Permission::WriteLnNode)?;

    let client = app_state.ln_node_client.as_breez_client()?;
    client.sync().await?;

    Ok(())
}

/// Backup node channels
///
/// Returns the static channel backup file contaning the channel information needed to recover funds for a Core Lightning node. See [the documentation](https://docs.corelightning.org/docs/backup#static-channel-backup)
#[utoipa::path(
    get,
    path = "/backup",
    tag = "Lightning Node",
    context_path = CONTEXT_PATH,
    responses(
        (status = 200, description = "Backup Downloaded", content_type = "text/plain"),
        (status = 401, description = "Unauthorized", body = ErrorResponse, example = json!(UNAUTHORIZED_EXAMPLE)),
        (status = 403, description = "Forbidden", body = ErrorResponse, example = json!(FORBIDDEN_EXAMPLE)),
        (status = 500, description = "Internal Server Error", body = ErrorResponse, example = json!(INTERNAL_EXAMPLE))
    )
)]
async fn backup(
    State(app_state): State<Arc<AppState>>,
    user: User,
) -> Result<Response, ApplicationError> {
    user.check_permission(Permission::ReadLnNode)?;

    let client = app_state.ln_node_client.as_breez_client()?;
    let data = client.backup()?;

    match data {
        Some(data) => {
            let filename = "channels_backup.txt";
            let body = Body::from(data.join("\n").into_bytes());

            let headers = [
                (header::CONTENT_TYPE, "text/plain"),
                (
                    header::CONTENT_DISPOSITION,
                    &format!("attachment; filename=\"{}\"", filename),
                ),
            ];

            Ok((headers, body).into_response())
        }
        None => Err(LightningError::Backup("No backup data found".to_string()))?,
    }
}
