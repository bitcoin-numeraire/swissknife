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

    let rgb_client = &app_state.rgb_client;

    let address = match rgb_client.get_address().await {
        Ok(address) => address,
        Err(e) => {
            eprintln!("Error Fetching address: {:?}", e);
            return Err(e.into());
        }
    };

    println!("Address fetched: {}", address);

    Ok(address)
}

async fn get_balance(State(app_state): State<Arc<AppState>>) -> Result<String, ApplicationError> {
    println!("Fetching balance");

    let rgb_client = &app_state.rgb_client;

    let balance = match rgb_client.get_btc_balance().await {
        Ok(balance) => balance,
        Err(e) => {
            eprintln!("Error Fetching balance: {:?}", e);
            return Err(e.into());
        }
    };

    println!("Balance fetched: {}", balance);

    Ok(balance.to_string())
}

async fn unspents(
    State(app_state): State<Arc<AppState>>,
) -> Result<Json<Vec<Unspent>>, ApplicationError> {
    println!("Fetching unspents");

    let rgb_client = &app_state.rgb_client;

    let unspents = match rgb_client.list_unspents().await {
        Ok(unspents) => unspents,
        Err(e) => {
            eprintln!("Error Fetching balance: {:?}", e);
            return Err(e.into());
        }
    };

    println!("Unspents fetched: {}", unspents.len());

    Ok(unspents.into())
}

async fn send(
    State(app_state): State<Arc<AppState>>,
    Json(payload): Json<SendBTCRequest>,
) -> Result<String, ApplicationError> {
    println!("Sending BTC: {:?}", payload);

    let rgb_client = &app_state.rgb_client;

    let tx_id = match rgb_client
        .send_btc(payload.address, payload.amount, payload.fee_rate)
        .await
    {
        Ok(id) => id,
        Err(e) => {
            eprintln!("Error creating utxos: {:?}", e);
            return Err(e.into());
        }
    };

    println!("BTC sent, tx id: {}", tx_id);

    Ok(tx_id)
}

async fn drain(
    State(app_state): State<Arc<AppState>>,
    Json(payload): Json<DrainRequest>,
) -> Result<String, ApplicationError> {
    println!("Draining BTC: {:?}", payload);

    let rgb_client = &app_state.rgb_client;

    let tx_id = match rgb_client
        .drain_btc(payload.address, payload.fee_rate)
        .await
    {
        Ok(id) => id,
        Err(e) => {
            eprintln!("Error creating utxos: {:?}", e);
            return Err(e.into());
        }
    };

    println!("BTC drained, tx id: {}", tx_id);

    Ok(tx_id)
}

async fn prepare_issuance(
    State(app_state): State<Arc<AppState>>,
    Json(payload): Json<PrepareIssuanceRequest>,
) -> Result<String, ApplicationError> {
    println!("Preparing utxos: {:?}", payload);

    let rgb_client = &app_state.rgb_client;

    let n_utxos = match rgb_client.create_utxos(payload.fee_rate).await {
        Ok(id) => id,
        Err(e) => {
            eprintln!("Error creating utxos: {:?}", e);
            return Err(e.into());
        }
    };

    println!("UTXOs created: {}", n_utxos);

    Ok(n_utxos.to_string())
}

async fn issue_contract(
    State(app_state): State<Arc<AppState>>,
    Json(payload): Json<IssueContractRequest>,
) -> Result<Json<ContractResponse>, ApplicationError> {
    println!("Issuing contract: {:?}", payload);

    let rgb_client = &app_state.rgb_client;

    let contract_id = match rgb_client
        .issue_contract(RGBContract {
            ticker: payload.ticker,
            name: payload.name,
            precision: payload.precision,
            amounts: payload.amounts,
        })
        .await
    {
        Ok(id) => id,
        Err(e) => {
            eprintln!("Error issuing contract: {:?}", e);
            return Err(e.into());
        }
    };

    println!("Contract issued: {}", contract_id);

    let contract = ContractResponse { id: contract_id };

    Ok(contract.into())
}

async fn list_assets(
    State(app_state): State<Arc<AppState>>,
) -> Result<Json<Assets>, ApplicationError> {
    println!("Fetching assets");

    let rgb_client = &app_state.rgb_client;

    let assets = match rgb_client.list_assets().await {
        Ok(assets) => assets,
        Err(e) => {
            eprintln!("Error Fetching assets: {:?}", e);
            return Err(e.into());
        }
    };

    Ok(assets.into())
}

async fn get_asset(
    Path(id): Path<String>,
    State(app_state): State<Arc<AppState>>,
) -> Result<Json<Metadata>, ApplicationError> {
    println!("Fetching asset: {}", id);

    let rgb_client = &app_state.rgb_client;

    let asset = match rgb_client.get_asset(id).await {
        Ok(asset) => asset,
        Err(e) => {
            eprintln!("Error Fetching asset {:?}", e);
            return Err(e.into());
        }
    };

    Ok(asset.into())
}

async fn get_asset_balance(
    Path(id): Path<String>,
    State(app_state): State<Arc<AppState>>,
) -> Result<Json<Balance>, ApplicationError> {
    println!("Fetching asset balance: {}", id);

    let rgb_client = &app_state.rgb_client;

    let balance = match rgb_client.get_asset_balance(id).await {
        Ok(balance) => balance,
        Err(e) => {
            eprintln!("Error Fetching balance {:?}", e);
            return Err(e.into());
        }
    };

    Ok(balance.into())
}

async fn send_assets(
    Path(id): Path<String>,
    State(app_state): State<Arc<AppState>>,
    Json(payload): Json<SendAssetsRequest>,
) -> Result<String, ApplicationError> {
    println!("Sending asset: {} with payload {:?}", id, payload);

    let rgb_client = &app_state.rgb_client;

    let tx_id = match rgb_client
        .send(
            id,
            payload.recipients,
            true,
            payload.fee_rate,
            payload.min_confirmations,
        )
        .await
    {
        Ok(id) => id,
        Err(e) => {
            eprintln!("Error sending assets: {:?}", e);
            return Err(e.into());
        }
    };

    println!("Assets sent, tx id: {}", tx_id);

    Ok(tx_id)
}

async fn invoice(
    State(app_state): State<Arc<AppState>>,
    Json(payload): Json<InvoiceAssetRequest>,
) -> Result<Json<ReceiveData>, ApplicationError> {
    println!("Generating invoice  {:?}", payload);

    let rgb_client = &app_state.rgb_client;

    let invoice = match rgb_client
        .invoice(
            payload.asset_id,
            payload.amount,
            payload.duration_seconds,
            payload.transport_endpoints,
            payload.min_confirmations,
        )
        .await
    {
        Ok(id) => id,
        Err(e) => {
            eprintln!("Error generating invoice: {:?}", e);
            return Err(e.into());
        }
    };

    Ok(invoice.into())
}
