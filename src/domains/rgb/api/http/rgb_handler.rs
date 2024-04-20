use std::sync::Arc;

use axum::{
    extract::{Path, State},
    http::StatusCode,
    response::IntoResponse,
    routing::{get, post},
    Json, Router,
};
use rgb_lib::wallet::{Assets, Balance, Metadata, ReceiveData, Transfer, Unspent};
use tracing::{debug, info, trace};

use crate::{
    application::{
        dtos::{
            DrainRequest, InvoiceAssetRequest, IssueAssetRequest, PrepareIssuanceRequest,
            SendAssetsRequest, SendBTCRequest,
        },
        errors::ApplicationError,
    },
    domains::rgb::entities::{RGBAsset, RGBAssetType, RGBInvoiceType},
    infra::app::AppState,
};

pub struct RGBHandler;

impl RGBHandler {
    pub fn routes() -> Router<Arc<AppState>> {
        Router::new()
            .route("/contracts", get(list_assets))
            .route("/contracts/issue", post(issue_asset))
            .route("/contracts/invoice", post(invoice))
            .route("/contracts/:id", get(get_asset))
            .route("/contracts/:id/balance", get(get_asset_balance))
            .route("/contracts/:id/send", post(send_assets))
            .route("/wallet/address", get(get_address))
            .route("/wallet/unspents", get(unspents))
            .route("/wallet/balance", get(get_balance))
            .route("/wallet/prepare-issuance", post(prepare_issuance))
            .route("/wallet/send", post(send))
            .route("/wallet/drain", post(drain))
            .route("/wallet/refresh", post(refresh))
            .route("/wallet/list-transfers", post(list_transfers))
    }
}

async fn get_address(State(app_state): State<Arc<AppState>>) -> Result<String, ApplicationError> {
    trace!("Fetching address");

    let address = app_state.rgb.get_address().await?;

    debug!(address, "New address fetched");
    Ok(address)
}

async fn get_balance(State(app_state): State<Arc<AppState>>) -> Result<String, ApplicationError> {
    trace!("Fetching balance");

    let balance = app_state.rgb.get_btc_balance().await?;

    Ok(balance.to_string())
}

async fn unspents(
    State(app_state): State<Arc<AppState>>,
) -> Result<Json<Vec<Unspent>>, ApplicationError> {
    trace!("Fetching unspents");

    let unspents = app_state.rgb.list_unspents().await?;

    debug!(n_unspents = unspents.len(), "Unspents fetched");
    Ok(unspents.into())
}

async fn send(
    State(app_state): State<Arc<AppState>>,
    Json(payload): Json<SendBTCRequest>,
) -> Result<String, ApplicationError> {
    info!(payload = ?payload, "Sending BTC");

    let tx_id = app_state
        .rgb
        .send_btc(payload.address.clone(), payload.amount, payload.fee_rate)
        .await?;

    info!(tx_id, recipient = payload.address, "BTC sent successfully");
    Ok(tx_id)
}

async fn drain(
    State(app_state): State<Arc<AppState>>,
    Json(payload): Json<DrainRequest>,
) -> Result<String, ApplicationError> {
    println!("Draining BTC: {:?}", payload);

    let tx_id = app_state
        .rgb
        .drain_btc(payload.address, payload.fee_rate)
        .await?;

    println!("BTC drained, tx id: {}", tx_id);
    Ok(tx_id)
}

async fn prepare_issuance(
    State(app_state): State<Arc<AppState>>,
    Json(payload): Json<PrepareIssuanceRequest>,
) -> Result<String, ApplicationError> {
    info!("Preparing utxos");

    let n_utxos = app_state
        .rgb
        .create_utxos(payload.fee_rate.unwrap_or(1.0))
        .await?;

    info!(n_utxos, "UTXOs created successfully");
    Ok(n_utxos.to_string())
}

async fn issue_asset(
    State(app_state): State<Arc<AppState>>,
    Json(payload): Json<IssueAssetRequest>,
) -> Result<String, ApplicationError> {
    info!(asset_type = ?payload.asset_type, "Issuing asset");

    let amount = payload.amount.unwrap_or(1);

    let contract = RGBAsset {
        asset_type: payload.asset_type.clone(),
        ticker: payload.ticker.clone().unwrap_or("".to_string()),
        name: payload.name,
        details: payload.details,
        precision: payload.precision.unwrap_or(0),
        amounts: vec![amount],
        filename: payload.filename,
    };

    let asset_id = match payload.asset_type {
        RGBAssetType::NIA => app_state.rgb.issue_asset_nia(contract).await?,
        RGBAssetType::CFA => app_state.rgb.issue_asset_cfa(contract).await?,
        RGBAssetType::UDA => app_state.rgb.issue_asset_uda(contract).await?,
    };

    if let Some(recipient) = payload.recipient {
        let result = app_state
            .rgb
            .send(
                asset_id.clone(),
                recipient,
                true,
                payload.fee_rate.unwrap_or(1.0),
                amount,
            )
            .await?;

        info!(
            asset_id,
            tx_id = result.txid,
            "Asset issued and sent successfully"
        );
    } else {
        info!(asset_id, "Asset issued successfully");
    }

    Ok(asset_id)
}

async fn list_assets(
    State(app_state): State<Arc<AppState>>,
) -> Result<Json<Assets>, ApplicationError> {
    trace!("Fetching assets");

    let assets = app_state.rgb.list_assets().await?;

    Ok(assets.into())
}

async fn list_transfers(
    State(app_state): State<Arc<AppState>>,
) -> Result<Json<Vec<Transfer>>, ApplicationError> {
    trace!("Fetching asset transfers");

    let assets = app_state.rgb.list_transfers(None).await?;

    Ok(assets.into())
}

async fn get_asset(
    Path(id): Path<String>,
    State(app_state): State<Arc<AppState>>,
) -> Result<Json<Metadata>, ApplicationError> {
    trace!(id, "Fetching asset");

    let asset = app_state.rgb.get_asset(id).await?;

    Ok(asset.into())
}

async fn get_asset_balance(
    Path(id): Path<String>,
    State(app_state): State<Arc<AppState>>,
) -> Result<Json<Balance>, ApplicationError> {
    trace!(id, "Fetching asset balance");

    let balance = app_state.rgb.get_asset_balance(id).await?;

    Ok(balance.into())
}

async fn send_assets(
    Path(id): Path<String>,
    State(app_state): State<Arc<AppState>>,
    Json(payload): Json<SendAssetsRequest>,
) -> Result<String, ApplicationError> {
    info!(
        asset_id = id,
        recipient = payload.recipient,
        "Sending assets"
    );

    let result = app_state
        .rgb
        .send(
            id,
            payload.recipient,
            true,
            payload.fee_rate.unwrap_or(1.0),
            payload.amount.unwrap_or(1),
        )
        .await?;

    info!(tx_id = result.txid, "Assets sent successfully");
    Ok(result.txid)
}
async fn invoice(
    State(app_state): State<Arc<AppState>>,
    Json(payload): Json<InvoiceAssetRequest>,
) -> Result<Json<ReceiveData>, ApplicationError> {
    info!(invoice_type = ?payload.invoice_type, "Generating invoice");

    let invoice = match payload.invoice_type {
        RGBInvoiceType::BLIND => {
            app_state
                .rgb
                .blind_receive(payload.asset_id, payload.amount, payload.duration_seconds)
                .await?
        }
        RGBInvoiceType::WITNESS => {
            app_state
                .rgb
                .witness_receive(payload.asset_id, payload.amount, payload.duration_seconds)
                .await?
        }
    };

    info!(invoice = invoice.invoice, "Invoice generated successfully");
    Ok(invoice.into())
}

async fn refresh(State(app_state): State<Arc<AppState>>) -> impl IntoResponse {
    info!("Refreshing asset transfers");

    match app_state.rgb.refresh(None).await {
        Ok(_) => (StatusCode::NO_CONTENT, ()).into_response(),
        Err(e) => e.into_response(),
    }
}
