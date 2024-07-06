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
        dtos::RegisterLightningAddressRequest,
        errors::ApplicationError,
    },
    domains::{
        lightning::entities::{LnAddress, LnAddressFilter},
        users::entities::{AuthUser, Permission},
    },
    infra::app::AppState,
};

#[derive(OpenApi)]
#[openapi(
    paths(register, get_one, list, delete_one, delete_many),
    components(schemas(LnAddress, RegisterLightningAddressRequest)),
    tags(
        (name = "Lightning Addresses", description = "LN Address management endpoints. Require authorization.")
    ),
    security(("jwt" = ["read:ln_address", "write:ln_address"]))
)]
pub struct LnAddressHandler;
pub const CONTEXT_PATH: &str = "/api/lightning/addresses";

pub fn ln_address_router() -> Router<Arc<AppState>> {
    Router::new()
        .route("/", get(list))
        .route("/", post(register))
        .route("/:id", get(get_one))
        .route("/:id", delete(delete_one))
        .route("/", delete(delete_many))
}

/// Register a new LN Address
///
/// Registers an address. Returns the address details. LN Addresses are ready to receive funds through the LNURL protocol upon registration.
#[utoipa::path(
    post,
    path = "",
    tag = "Lightning Addresses",
    context_path = CONTEXT_PATH,
    request_body = RegisterLightningAddressRequest,
    responses(
        (status = 200, description = "LN Address Registered", body = LnAddress),
        (status = 400, description = "Bad Request", body = ErrorResponse, example = json!(BAD_REQUEST_EXAMPLE)),
        (status = 401, description = "Unauthorized", body = ErrorResponse, example = json!(UNAUTHORIZED_EXAMPLE)),
        (status = 403, description = "Forbidden", body = ErrorResponse, example = json!(FORBIDDEN_EXAMPLE)),
        (status = 422, description = "Unprocessable Entity", body = ErrorResponse, example = json!(UNPROCESSABLE_EXAMPLE)),
        (status = 500, description = "Internal Server Error", body = ErrorResponse, example = json!(INTERNAL_EXAMPLE))
    )
)]
async fn register(
    State(app_state): State<Arc<AppState>>,
    user: AuthUser,
    Json(payload): Json<RegisterLightningAddressRequest>,
) -> Result<Json<LnAddress>, ApplicationError> {
    user.check_permission(Permission::WriteLnAddress)?;

    let ln_address = app_state
        .services
        .lnurl
        .register(payload.user_id.unwrap_or(user.sub), payload.username)
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
async fn get_one(
    State(app_state): State<Arc<AppState>>,
    user: AuthUser,
    Path(id): Path<Uuid>,
) -> Result<Json<LnAddress>, ApplicationError> {
    user.check_permission(Permission::ReadLnAddress)?;

    let ln_address = app_state.services.lnurl.get(id).await?;
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
async fn list(
    State(app_state): State<Arc<AppState>>,
    user: AuthUser,
    Query(query_params): Query<LnAddressFilter>,
) -> Result<Json<Vec<LnAddress>>, ApplicationError> {
    user.check_permission(Permission::ReadLnAddress)?;

    let ln_addresses = app_state.services.lnurl.list(query_params).await?;

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
async fn delete_one(
    State(app_state): State<Arc<AppState>>,
    user: AuthUser,
    Path(id): Path<Uuid>,
) -> Result<(), ApplicationError> {
    user.check_permission(Permission::WriteLnAddress)?;

    app_state.services.lnurl.delete(id).await?;
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
async fn delete_many(
    State(app_state): State<Arc<AppState>>,
    user: AuthUser,
    Query(query_params): Query<LnAddressFilter>,
) -> Result<Json<u64>, ApplicationError> {
    user.check_permission(Permission::WriteLnAddress)?;

    let n_deleted = app_state.services.lnurl.delete_many(query_params).await?;
    Ok(n_deleted.into())
}
