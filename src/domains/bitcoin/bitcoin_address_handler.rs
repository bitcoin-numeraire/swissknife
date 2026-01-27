use std::sync::Arc;

use axum::{
    extract::State,
    routing::{delete, get, post},
    Router,
};
use utoipa::OpenApi;
use uuid::Uuid;

use crate::{
    application::{
        docs::{
            BAD_REQUEST_EXAMPLE, FORBIDDEN_EXAMPLE, INTERNAL_EXAMPLE, NOT_FOUND_EXAMPLE, UNAUTHORIZED_EXAMPLE,
            UNPROCESSABLE_EXAMPLE,
        },
        dtos::{BtcAddressResponse, ErrorResponse, NewBtcAddressRequest},
        entities::AppServices,
        errors::ApplicationError,
    },
    domains::{
        bitcoin::{BtcAddressFilter, BtcAddressType, BtcNetwork},
        user::{Permission, User},
    },
    infra::axum::{Json, Path, Query},
};

#[derive(OpenApi)]
#[openapi(
    paths(generate_btc_address, list_btc_addresses, get_btc_address, delete_btc_address, delete_btc_addresses),
    components(schemas(NewBtcAddressRequest, BtcAddressResponse, BtcNetwork, BtcAddressResponse, BtcAddressType)),
    tags(
        (name = "Bitcoin Addresses", description = "Bitcoin Address management endpoints. Require `read:btc_address` or `write:btc_address` permissions.")
    ),
)]
pub struct BtcAddressHandler;
pub const CONTEXT_PATH: &str = "/v1/bitcoin/addresses";

pub fn router() -> Router<Arc<AppServices>> {
    Router::new()
        .route("/", post(generate_btc_address))
        .route("/", get(list_btc_addresses))
        .route("/:id", get(get_btc_address))
        .route("/:id", delete(delete_btc_address))
        .route("/", delete(delete_btc_addresses))
}

/// Generate a new Bitcoin address
///
/// Returns the generated Bitcoin address for the given user
#[utoipa::path(
    post,
    path = "",
    tag = "Bitcoin Addresses",
    context_path = CONTEXT_PATH,
    request_body = NewBtcAddressRequest,
    responses(
        (status = 200, description = "Bitcoin Address Created", body = BtcAddressResponse),
        (status = 400, description = "Bad Request", body = ErrorResponse, example = json!(BAD_REQUEST_EXAMPLE)),
        (status = 401, description = "Unauthorized", body = ErrorResponse, example = json!(UNAUTHORIZED_EXAMPLE)),
        (status = 403, description = "Forbidden", body = ErrorResponse, example = json!(FORBIDDEN_EXAMPLE)),
        (status = 422, description = "Unprocessable Entity", body = ErrorResponse, example = json!(UNPROCESSABLE_EXAMPLE)),
        (status = 500, description = "Internal Server Error", body = ErrorResponse, example = json!(INTERNAL_EXAMPLE))
    )
)]
async fn generate_btc_address(
    State(services): State<Arc<AppServices>>,
    user: User,
    Json(payload): Json<NewBtcAddressRequest>,
) -> Result<Json<BtcAddressResponse>, ApplicationError> {
    user.check_permission(Permission::WriteBtcAddress)?;

    let address = services
        .bitcoin
        .new_deposit_address(payload.wallet_id.unwrap_or(user.wallet_id), payload.address_type)
        .await?;
    Ok(Json(address.into()))
}

/// Find a Bitcoin address
///
/// Returns the Bitcoin address by its ID.
#[utoipa::path(
    get,
    path = "/{id}",
    tag = "Bitcoin Addresses",
    context_path = CONTEXT_PATH,
    responses(
        (status = 200, description = "Found", body = BtcAddressResponse),
        (status = 400, description = "Bad Request", body = ErrorResponse, example = json!(BAD_REQUEST_EXAMPLE)),
        (status = 401, description = "Unauthorized", body = ErrorResponse, example = json!(UNAUTHORIZED_EXAMPLE)),
        (status = 403, description = "Forbidden", body = ErrorResponse, example = json!(FORBIDDEN_EXAMPLE)),
        (status = 404, description = "Not Found", body = ErrorResponse, example = json!(NOT_FOUND_EXAMPLE)),
        (status = 500, description = "Internal Server Error", body = ErrorResponse, example = json!(INTERNAL_EXAMPLE))
    )
)]
async fn get_btc_address(
    State(services): State<Arc<AppServices>>,
    user: User,
    Path(id): Path<Uuid>,
) -> Result<Json<BtcAddressResponse>, ApplicationError> {
    user.check_permission(Permission::ReadBtcAddress)?;

    let address = services.bitcoin.get_address(id).await?;
    Ok(Json(address.into()))
}

/// List Bitcoin addresses
///
/// Returns all the Bitcoin addresses given a filter
#[utoipa::path(
    get,
    path = "",
    tag = "Bitcoin Addresses",
    context_path = CONTEXT_PATH,
    params(BtcAddressFilter),
    responses(
        (status = 200, description = "Success", body = Vec<BtcAddressResponse>),
        (status = 400, description = "Bad Request", body = ErrorResponse, example = json!(BAD_REQUEST_EXAMPLE)),
        (status = 401, description = "Unauthorized", body = ErrorResponse, example = json!(UNAUTHORIZED_EXAMPLE)),
        (status = 403, description = "Forbidden", body = ErrorResponse, example = json!(FORBIDDEN_EXAMPLE)),
        (status = 500, description = "Internal Server Error", body = ErrorResponse, example = json!(INTERNAL_EXAMPLE))
    )
)]
async fn list_btc_addresses(
    State(services): State<Arc<AppServices>>,
    user: User,
    Query(filter): Query<BtcAddressFilter>,
) -> Result<Json<Vec<BtcAddressResponse>>, ApplicationError> {
    user.check_permission(Permission::ReadBtcAddress)?;

    let addresses = services.bitcoin.list_addresses(filter).await?;
    let response: Vec<BtcAddressResponse> = addresses.into_iter().map(Into::into).collect();

    Ok(response.into())
}

/// Delete a Bitcoin address
///
/// Deletes an Bitcoin address by ID. Returns an empty body. Deleting a Bitcoin address has an effect on the user balance
#[utoipa::path(
    delete,
    path = "/{id}",
    tag = "Bitcoin Addresses",
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
async fn delete_btc_address(
    State(services): State<Arc<AppServices>>,
    user: User,
    Path(id): Path<Uuid>,
) -> Result<(), ApplicationError> {
    user.check_permission(Permission::WriteBtcAddress)?;

    services.bitcoin.delete_address(id).await?;
    Ok(())
}

/// Delete Bitcoin addresses
///
/// Deletes all the Bitcoin addresses given a filter. Returns the number of deleted addresses. Deleting an address can have an effect on the user balance
#[utoipa::path(
    delete,
    path = "",
    tag = "Bitcoin Addresses",
    context_path = CONTEXT_PATH,
    params(BtcAddressFilter),
    responses(
        (status = 200, description = "Success", body = u64),
        (status = 400, description = "Bad Request", body = ErrorResponse, example = json!(BAD_REQUEST_EXAMPLE)),
        (status = 401, description = "Unauthorized", body = ErrorResponse, example = json!(UNAUTHORIZED_EXAMPLE)),
        (status = 403, description = "Forbidden", body = ErrorResponse, example = json!(FORBIDDEN_EXAMPLE)),
        (status = 500, description = "Internal Server Error", body = ErrorResponse, example = json!(INTERNAL_EXAMPLE))
    )
)]
async fn delete_btc_addresses(
    State(services): State<Arc<AppServices>>,
    user: User,
    Query(query_params): Query<BtcAddressFilter>,
) -> Result<Json<u64>, ApplicationError> {
    user.check_permission(Permission::WriteBtcAddress)?;

    let n_deleted = services.bitcoin.delete_many_addresses(query_params).await?;
    Ok(n_deleted.into())
}
