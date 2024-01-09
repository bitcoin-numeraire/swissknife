use std::sync::Arc;

use axum::{
    extract::{Path, State},
    routing::get,
    routing::post,
    Json, Router,
};
use rgb_lib::wallet::{Assets, Balance, Metadata, ReceiveData, Unspent};

use crate::{
    adapters::app::AppState,
    application::{
        dtos::{
            ContractResponse, DrainRequest, InvoiceAssetRequest, IssueContractRequest,
            PrepareIssuanceRequest, SendAssetsRequest, SendBTCRequest,
        },
        errors::ApplicationError,
    },
    domains::rgb::entities::RGBContract,
};

pub struct RGBHandler;

impl RGBHandler {
    pub fn routes() -> Router<Arc<AppState>> {
        Router::new()
            .route("/contracts", get(list_assets))
            .route("/contracts/issue", post(issue_contract))
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
    }
}

async fn get_address(State(app_state): State<Arc<AppState>>) -> Result<String, ApplicationError> {
    println!("Fetching address");

    let address = app_state.rgb.get_address().await?;

    println!("Address fetched: {}", address);
    Ok(address)
}

async fn get_balance(State(app_state): State<Arc<AppState>>) -> Result<String, ApplicationError> {
    println!("Fetching balance");

    let balance = app_state.rgb.get_btc_balance().await?;

    println!("Balance fetched: {}", balance);
    Ok(balance.to_string())
}

async fn unspents(
    State(app_state): State<Arc<AppState>>,
) -> Result<Json<Vec<Unspent>>, ApplicationError> {
    println!("Fetching unspents");

    let unspents = app_state.rgb.list_unspents().await?;

    println!("Unspents fetched: {}", unspents.len());
    Ok(unspents.into())
}

async fn send(
    State(app_state): State<Arc<AppState>>,
    Json(payload): Json<SendBTCRequest>,
) -> Result<String, ApplicationError> {
    println!("Sending BTC: {:?}", payload);

    let tx_id = app_state
        .rgb
        .send_btc(payload.address, payload.amount, payload.fee_rate)
        .await?;

    println!("BTC sent, tx id: {}", tx_id);
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
    println!("Preparing utxos: {:?}", payload);

    let n_utxos = app_state.rgb.create_utxos(payload.fee_rate).await?;

    println!("UTXOs created: {}", n_utxos);
    Ok(n_utxos.to_string())
}

async fn issue_contract(
    State(app_state): State<Arc<AppState>>,
    Json(payload): Json<IssueContractRequest>,
) -> Result<Json<ContractResponse>, ApplicationError> {
    println!("Issuing contract: {:?}", payload);

    let contract_id = app_state
        .rgb
        .issue_contract(RGBContract {
            ticker: payload.ticker,
            name: payload.name,
            precision: payload.precision,
            amounts: payload.amounts,
        })
        .await?;

    println!("Contract issued: {}", contract_id);
    let contract = ContractResponse { id: contract_id };

    Ok(contract.into())
}

async fn list_assets(
    State(app_state): State<Arc<AppState>>,
) -> Result<Json<Assets>, ApplicationError> {
    println!("Fetching assets");

    let assets = app_state.rgb.list_assets().await?;

    Ok(assets.into())
}

async fn get_asset(
    Path(id): Path<String>,
    State(app_state): State<Arc<AppState>>,
) -> Result<Json<Metadata>, ApplicationError> {
    println!("Fetching asset: {}", id);

    let asset = app_state.rgb.get_asset(id).await?;

    Ok(asset.into())
}

async fn get_asset_balance(
    Path(id): Path<String>,
    State(app_state): State<Arc<AppState>>,
) -> Result<Json<Balance>, ApplicationError> {
    println!("Fetching asset balance: {}", id);

    let balance = app_state.rgb.get_asset_balance(id).await?;

    Ok(balance.into())
}

async fn send_assets(
    Path(id): Path<String>,
    State(app_state): State<Arc<AppState>>,
    Json(payload): Json<SendAssetsRequest>,
) -> Result<String, ApplicationError> {
    println!("Sending asset: {} with payload {:?}", id, payload);

    let tx_id = app_state
        .rgb
        .send(
            id,
            payload.recipients,
            true,
            payload.fee_rate,
            payload.min_confirmations,
        )
        .await?;

    println!("Assets sent, tx id: {}", tx_id);
    Ok(tx_id)
}

async fn invoice(
    State(app_state): State<Arc<AppState>>,
    Json(payload): Json<InvoiceAssetRequest>,
) -> Result<Json<ReceiveData>, ApplicationError> {
    println!("Generating invoice  {:?}", payload);

    let invoice = app_state
        .rgb
        .invoice(
            payload.asset_id,
            payload.amount,
            payload.duration_seconds,
            payload.transport_endpoints,
            payload.min_confirmations,
        )
        .await?;

    Ok(invoice.into())
}
