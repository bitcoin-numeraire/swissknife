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
            BAD_REQUEST_EXAMPLE, FORBIDDEN_EXAMPLE, INTERNAL_EXAMPLE, NOT_FOUND_EXAMPLE,
            UNAUTHORIZED_EXAMPLE, UNPROCESSABLE_EXAMPLE,
        },
        dtos::RegisterLnAddressRequest,
        errors::ApplicationError,
    },
    domains::user::{Permission, User},
    infra::app::AppState,
};

use super::{LnAddress, LnAddressFilter};

#[derive(OpenApi)]
#[openapi(
    paths(register_address, get_address, list_addresses, delete_address, delete_addresses),
    components(schemas(LnAddress, RegisterLnAddressRequest)),
    tags(
        (name = "Lightning Addresses", description = "LN Address management endpoints as defined in the [protocol specification](https://lightningaddress.com/). Require `read:ln_address` or `write:ln_address` permissions.")
    )
)]
pub struct LnAddressHandler;
pub const CONTEXT_PATH: &str = "/v1/lightning-addresses";

pub fn router() -> Router<Arc<AppState>> {
    Router::new()
        .route("/", get(list_addresses))
        .route("/", post(register_address))
        .route("/:id", get(get_address))
        .route("/:id", delete(delete_address))
        .route("/", delete(delete_addresses))
}

/// Register a new LN Address
///
/// Registers an address. Returns the address details. LN Addresses are ready to receive funds through the LNURL protocol upon registration.
#[utoipa::path(
    post,
    path = "",
    tag = "Lightning Addresses",
    context_path = CONTEXT_PATH,
    request_body = RegisterLnAddressRequest,
    responses(
        (status = 200, description = "LN Address Registered", body = LnAddress),
        (status = 400, description = "Bad Request", body = ErrorResponse, example = json!(BAD_REQUEST_EXAMPLE)),
        (status = 401, description = "Unauthorized", body = ErrorResponse, example = json!(UNAUTHORIZED_EXAMPLE)),
        (status = 403, description = "Forbidden", body = ErrorResponse, example = json!(FORBIDDEN_EXAMPLE)),
        (status = 422, description = "Unprocessable Entity", body = ErrorResponse, example = json!(UNPROCESSABLE_EXAMPLE)),
        (status = 500, description = "Internal Server Error", body = ErrorResponse, example = json!(INTERNAL_EXAMPLE))
    )
)]
async fn register_address(
    State(app_state): State<Arc<AppState>>,
    user: User,
    Json(payload): Json<RegisterLnAddressRequest>,
) -> Result<Json<LnAddress>, ApplicationError> {
    user.check_permission(Permission::WriteLnAddress)?;

    let ln_address = app_state
        .services
        .ln_address
        .register(
            payload.wallet_id.unwrap_or(user.wallet_id),
            payload.username,
            payload.allows_nostr,
            payload.nostr_pubkey,
        )
        .await?;
    Ok(ln_address.into())
}

/// Find a LN Address
///
/// Returns the address by its ID.
#[utoipa::path(
    get,
    path = "/{id}",
    tag = "Lightning Addresses",
    context_path = CONTEXT_PATH,
    responses(
        (status = 200, description = "Found", body = LnAddress),
        (status = 400, description = "Bad Request", body = ErrorResponse, example = json!(BAD_REQUEST_EXAMPLE)),
        (status = 401, description = "Unauthorized", body = ErrorResponse, example = json!(UNAUTHORIZED_EXAMPLE)),
        (status = 403, description = "Forbidden", body = ErrorResponse, example = json!(FORBIDDEN_EXAMPLE)),
        (status = 404, description = "Not Found", body = ErrorResponse, example = json!(NOT_FOUND_EXAMPLE)),
        (status = 500, description = "Internal Server Error", body = ErrorResponse, example = json!(INTERNAL_EXAMPLE))
    )
)]
async fn get_address(
    State(app_state): State<Arc<AppState>>,
    user: User,
    Path(id): Path<Uuid>,
) -> Result<Json<LnAddress>, ApplicationError> {
    user.check_permission(Permission::ReadLnAddress)?;

    let ln_address = app_state.services.ln_address.get(id).await?;
    Ok(ln_address.into())
}

/// List LN Addresses
///
/// Returns all the addresses given a filter
#[utoipa::path(
    get,
    path = "",
    tag = "Lightning Addresses",
    context_path = CONTEXT_PATH,
    params(LnAddressFilter),
    responses(
        (status = 200, description = "Success", body = Vec<LnAddress>),
        (status = 400, description = "Bad Request", body = ErrorResponse, example = json!(BAD_REQUEST_EXAMPLE)),
        (status = 401, description = "Unauthorized", body = ErrorResponse, example = json!(UNAUTHORIZED_EXAMPLE)),
        (status = 403, description = "Forbidden", body = ErrorResponse, example = json!(FORBIDDEN_EXAMPLE)),
        (status = 500, description = "Internal Server Error", body = ErrorResponse, example = json!(INTERNAL_EXAMPLE))
    )
)]
async fn list_addresses(
    State(app_state): State<Arc<AppState>>,
    user: User,
    Query(query_params): Query<LnAddressFilter>,
) -> Result<Json<Vec<LnAddress>>, ApplicationError> {
    user.check_permission(Permission::ReadLnAddress)?;

    let ln_addresses = app_state.services.ln_address.list(query_params).await?;

    let response: Vec<LnAddress> = ln_addresses.into_iter().map(Into::into).collect();

    Ok(response.into())
}

/// Delete a LN Address
///
/// Deletes an address by ID. Returns an empty body
#[utoipa::path(
    delete,
    path = "/{id}",
    tag = "Lightning Addresses",
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
async fn delete_address(
    State(app_state): State<Arc<AppState>>,
    user: User,
    Path(id): Path<Uuid>,
) -> Result<(), ApplicationError> {
    user.check_permission(Permission::WriteLnAddress)?;

    app_state.services.ln_address.delete(id).await?;
    Ok(())
}

/// Delete LN Addresses
///
/// Deletes all the addresses given a filter. Returns the number of deleted addresses
#[utoipa::path(
    delete,
    path = "",
    tag = "Lightning Addresses",
    context_path = CONTEXT_PATH,
    params(LnAddressFilter),
    responses(
        (status = 200, description = "Success", body = u64),
        (status = 400, description = "Bad Request", body = ErrorResponse, example = json!(BAD_REQUEST_EXAMPLE)),
        (status = 401, description = "Unauthorized", body = ErrorResponse, example = json!(UNAUTHORIZED_EXAMPLE)),
        (status = 403, description = "Forbidden", body = ErrorResponse, example = json!(FORBIDDEN_EXAMPLE)),
        (status = 500, description = "Internal Server Error", body = ErrorResponse, example = json!(INTERNAL_EXAMPLE))
    )
)]
async fn delete_addresses(
    State(app_state): State<Arc<AppState>>,
    user: User,
    Query(query_params): Query<LnAddressFilter>,
) -> Result<Json<u64>, ApplicationError> {
    user.check_permission(Permission::WriteLnAddress)?;

    let n_deleted = app_state
        .services
        .ln_address
        .delete_many(query_params)
        .await?;
    Ok(n_deleted.into())
}
