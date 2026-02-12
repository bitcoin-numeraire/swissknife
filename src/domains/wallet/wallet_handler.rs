use std::sync::Arc;

use axum::{
    extract::{Path, State},
    routing::{delete, get, post},
    Json, Router,
};
use axum_extra::extract::Query;
use utoipa::OpenApi;
use uuid::Uuid;

use crate::{
    application::{
        docs::{
            BAD_REQUEST_EXAMPLE, FORBIDDEN_EXAMPLE, INTERNAL_EXAMPLE, NOT_FOUND_EXAMPLE, UNAUTHORIZED_EXAMPLE,
            UNPROCESSABLE_EXAMPLE,
        },
        dtos::{ErrorResponse, RegisterWalletRequest, WalletResponse},
        entities::AppServices,
        errors::ApplicationError,
    },
    domains::user::{Permission, User},
};

use super::{WalletFilter, WalletOverview};

#[derive(OpenApi)]
#[openapi(
    paths(register_wallet, list_wallets, list_wallet_overviews, get_wallet, delete_wallet, delete_wallets),
    components(schemas(WalletOverview, RegisterWalletRequest)),
    tags(
        (name = "Wallets", description = "Wallet management endpoints. Require `read:wallet` or `write:wallet` permissions.")
    ),
)]
pub struct WalletHandler;
pub const CONTEXT_PATH: &str = "/v1/wallets";

pub fn router() -> Router<Arc<AppServices>> {
    Router::new()
        .route("/", post(register_wallet))
        .route("/", get(list_wallets))
        .route("/overviews", get(list_wallet_overviews))
        .route("/{id}", get(get_wallet))
        .route("/{id}", delete(delete_wallet))
        .route("/", delete(delete_wallets))
}

/// Register a new wallet
///
/// Returns the generated wallet for the given user
#[utoipa::path(
    post,
    path = "",
    tag = "Wallets",
    context_path = CONTEXT_PATH,
    request_body = RegisterWalletRequest,
    responses(
        (status = 200, description = "Wallet Created", body = WalletResponse),
        (status = 400, description = "Bad Request", body = ErrorResponse, example = json!(BAD_REQUEST_EXAMPLE)),
        (status = 401, description = "Unauthorized", body = ErrorResponse, example = json!(UNAUTHORIZED_EXAMPLE)),
        (status = 403, description = "Forbidden", body = ErrorResponse, example = json!(FORBIDDEN_EXAMPLE)),
        (status = 422, description = "Unprocessable Entity", body = ErrorResponse, example = json!(UNPROCESSABLE_EXAMPLE)),
        (status = 500, description = "Internal Server Error", body = ErrorResponse, example = json!(INTERNAL_EXAMPLE))
    )
)]
async fn register_wallet(
    State(services): State<Arc<AppServices>>,
    user: User,
    Json(payload): Json<RegisterWalletRequest>,
) -> Result<Json<WalletResponse>, ApplicationError> {
    user.check_permission(Permission::WriteWallet)?;

    let wallet = services.wallet.register(payload.user_id).await?;
    Ok(Json(wallet.into()))
}

/// List wallets
///
/// Returns all the wallets without any linked data. Use the wallet ID to get the full wallet details.
#[utoipa::path(
    get,
    path = "",
    tag = "Wallets",
    context_path = CONTEXT_PATH,
    responses(
        (status = 200, description = "Success", body = Vec<WalletResponse>),
        (status = 400, description = "Bad Request", body = ErrorResponse, example = json!(BAD_REQUEST_EXAMPLE)),
        (status = 401, description = "Unauthorized", body = ErrorResponse, example = json!(UNAUTHORIZED_EXAMPLE)),
        (status = 403, description = "Forbidden", body = ErrorResponse, example = json!(FORBIDDEN_EXAMPLE)),
        (status = 500, description = "Internal Server Error", body = ErrorResponse, example = json!(INTERNAL_EXAMPLE))
    )
)]
async fn list_wallets(
    State(services): State<Arc<AppServices>>,
    user: User,
    Query(filter): Query<WalletFilter>,
) -> Result<Json<Vec<WalletResponse>>, ApplicationError> {
    user.check_permission(Permission::ReadWallet)?;

    let wallets = services.wallet.list(filter).await?;
    let response: Vec<WalletResponse> = wallets.into_iter().map(Into::into).collect();

    Ok(response.into())
}

/// List wallet overviews
///
/// Returns all the wallet overviews. A wallet overview is a summary of a wallet with the number of payments, invoices and contacts.
#[utoipa::path(
    get,
    path = "/overviews",
    tag = "Wallets",
    context_path = CONTEXT_PATH,
    responses(
        (status = 200, description = "Success", body = Vec<WalletOverview>),
        (status = 400, description = "Bad Request", body = ErrorResponse, example = json!(BAD_REQUEST_EXAMPLE)),
        (status = 401, description = "Unauthorized", body = ErrorResponse, example = json!(UNAUTHORIZED_EXAMPLE)),
        (status = 403, description = "Forbidden", body = ErrorResponse, example = json!(FORBIDDEN_EXAMPLE)),
        (status = 500, description = "Internal Server Error", body = ErrorResponse, example = json!(INTERNAL_EXAMPLE))
    )
)]
async fn list_wallet_overviews(
    State(services): State<Arc<AppServices>>,
    user: User,
) -> Result<Json<Vec<WalletOverview>>, ApplicationError> {
    user.check_permission(Permission::ReadWallet)?;

    let overviews = services.wallet.list_overviews().await?;

    Ok(Json(overviews))
}

/// Find a wallet
///
/// Returns the wallet by its ID.
#[utoipa::path(
    get,
    path = "/{id}",
    tag = "Wallets",
    context_path = CONTEXT_PATH,
    responses(
        (status = 200, description = "Found", body = WalletResponse),
        (status = 400, description = "Bad Request", body = ErrorResponse, example = json!(BAD_REQUEST_EXAMPLE)),
        (status = 401, description = "Unauthorized", body = ErrorResponse, example = json!(UNAUTHORIZED_EXAMPLE)),
        (status = 403, description = "Forbidden", body = ErrorResponse, example = json!(FORBIDDEN_EXAMPLE)),
        (status = 404, description = "Not Found", body = ErrorResponse, example = json!(NOT_FOUND_EXAMPLE)),
        (status = 500, description = "Internal Server Error", body = ErrorResponse, example = json!(INTERNAL_EXAMPLE))
    )
)]
async fn get_wallet(
    State(services): State<Arc<AppServices>>,
    user: User,
    Path(id): Path<Uuid>,
) -> Result<Json<WalletResponse>, ApplicationError> {
    user.check_permission(Permission::ReadWallet)?;

    let wallet = services.wallet.get(id).await?;
    Ok(Json(wallet.into()))
}

/// Delete a wallet
///
/// Deletes an wallet by ID. Returns an empty body. Deleting a wallet removes all data related to that wallet.
#[utoipa::path(
    delete,
    path = "/{id}",
    tag = "Wallets",
    context_path = CONTEXT_PATH,
    responses(
        (status = 200, description = "Deleted"),
        (status = 400, description = "Bad Request", body = ErrorResponse, example = json!(BAD_REQUEST_EXAMPLE)),
        (status = 401, description = "Unauthorized", body = ErrorResponse, example = json!(UNAUTHORIZED_EXAMPLE)),
        (status = 403, description = "Forbidden", body = ErrorResponse, example = json!(FORBIDDEN_EXAMPLE)),
        (status = 404, description = "Not Found", body = ErrorResponse, example = json!(NOT_FOUND_EXAMPLE)),
        (status = 500, description = "Internal Server Error", body = ErrorResponse, example = json!(INTERNAL_EXAMPLE))
    )
)]
async fn delete_wallet(
    State(services): State<Arc<AppServices>>,
    user: User,
    Path(id): Path<Uuid>,
) -> Result<(), ApplicationError> {
    user.check_permission(Permission::WriteWallet)?;

    services.wallet.delete(id).await?;
    Ok(())
}

/// Delete wallets
///
/// Deletes all the wallets given a filter. Returns the number of deleted wallets. Deleting a wallet removes all data related to that wallet.
#[utoipa::path(
    delete,
    path = "",
    tag = "Wallets",
    context_path = CONTEXT_PATH,
    params(WalletFilter),
    responses(
        (status = 200, description = "Success", body = u64),
        (status = 400, description = "Bad Request", body = ErrorResponse, example = json!(BAD_REQUEST_EXAMPLE)),
        (status = 401, description = "Unauthorized", body = ErrorResponse, example = json!(UNAUTHORIZED_EXAMPLE)),
        (status = 403, description = "Forbidden", body = ErrorResponse, example = json!(FORBIDDEN_EXAMPLE)),
        (status = 500, description = "Internal Server Error", body = ErrorResponse, example = json!(INTERNAL_EXAMPLE))
    )
)]
async fn delete_wallets(
    State(services): State<Arc<AppServices>>,
    user: User,
    Query(query_params): Query<WalletFilter>,
) -> Result<Json<u64>, ApplicationError> {
    user.check_permission(Permission::WriteWallet)?;

    let n_deleted = services.wallet.delete_many(query_params).await?;
    Ok(n_deleted.into())
}
