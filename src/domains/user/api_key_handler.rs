use std::sync::Arc;

use axum::{
    extract::State,
    routing::{delete, get, post},
    Router,
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
        dtos::{ApiKeyResponse, CreateApiKeyRequest, ErrorResponse},
        errors::ApplicationError,
    },
    infra::{
        app::AppState,
        axum::{Json, Path},
    },
};

use super::{ApiKeyFilter, Permission, User};

#[derive(OpenApi)]
#[openapi(
    paths(create_api_key, get_api_key, list_api_keys, revoke_api_key, revoke_api_keys),
    components(schemas(CreateApiKeyRequest, ApiKeyResponse, Permission)),
    tags(
        (name = "API Keys", description = "API Key Management. Require `read:api_key` or `write:api_key` permissions. "),
    )
)]
pub struct ApiKeyHandler;
pub const CONTEXT_PATH: &str = "/v1/api-keys";

pub fn api_key_router() -> Router<Arc<AppState>> {
    Router::new()
        .route("/", post(create_api_key))
        .route("/", get(list_api_keys))
        .route("/:id", get(get_api_key))
        .route("/:id", delete(revoke_api_key))
        .route("/", delete(revoke_api_keys))
}

/// Generate a new API Key
///
/// Returns the generated API Key for the given user. Users can create API keys with permissions as a subset of his current permissions.
#[utoipa::path(
    post,
    path = "",
    tag = "API Keys",
    context_path = CONTEXT_PATH,
    request_body = CreateApiKeyRequest,
    responses(
        (status = 200, description = "API Key Created", body = ApiKeyResponse),
        (status = 400, description = "Bad Request", body = ErrorResponse, example = json!(BAD_REQUEST_EXAMPLE)),
        (status = 401, description = "Unauthorized", body = ErrorResponse, example = json!(UNAUTHORIZED_EXAMPLE)),
        (status = 403, description = "Forbidden", body = ErrorResponse, example = json!(FORBIDDEN_EXAMPLE)),
        (status = 422, description = "Unprocessable Entity", body = ErrorResponse, example = json!(UNPROCESSABLE_EXAMPLE)),
        (status = 500, description = "Internal Server Error", body = ErrorResponse, example = json!(INTERNAL_EXAMPLE))
    )
)]
async fn create_api_key(
    State(app_state): State<Arc<AppState>>,
    user: User,
    Json(payload): Json<CreateApiKeyRequest>,
) -> Result<Json<ApiKeyResponse>, ApplicationError> {
    user.check_permission(Permission::WriteApiKey)?;

    let api_key = app_state.services.api_key.generate(user, payload).await?;
    Ok(Json(api_key.into()))
}

/// Find an API Key
///
/// Returns the API Key by its ID.
#[utoipa::path(
    get,
    path = "/{id}",
    tag = "API Keys",
    context_path = CONTEXT_PATH,
    responses(
        (status = 200, description = "Found", body = ApiKeyResponse),
        (status = 400, description = "Bad Request", body = ErrorResponse, example = json!(BAD_REQUEST_EXAMPLE)),
        (status = 401, description = "Unauthorized", body = ErrorResponse, example = json!(UNAUTHORIZED_EXAMPLE)),
        (status = 403, description = "Forbidden", body = ErrorResponse, example = json!(FORBIDDEN_EXAMPLE)),
        (status = 404, description = "Not Found", body = ErrorResponse, example = json!(NOT_FOUND_EXAMPLE)),
        (status = 500, description = "Internal Server Error", body = ErrorResponse, example = json!(INTERNAL_EXAMPLE))
    )
)]
async fn get_api_key(
    State(app_state): State<Arc<AppState>>,
    user: User,
    Path(id): Path<Uuid>,
) -> Result<Json<ApiKeyResponse>, ApplicationError> {
    user.check_permission(Permission::ReadApiKey)?;

    let api_key = app_state.services.api_key.get(id).await?;
    Ok(Json(api_key.into()))
}

/// List API Keys
///
/// Returns all the API Keys given a filter
#[utoipa::path(
    get,
    path = "",
    tag = "API Keys",
    context_path = CONTEXT_PATH,
    params(ApiKeyFilter),
    responses(
        (status = 200, description = "Success", body = Vec<ApiKeyResponse>),
        (status = 400, description = "Bad Request", body = ErrorResponse, example = json!(BAD_REQUEST_EXAMPLE)),
        (status = 401, description = "Unauthorized", body = ErrorResponse, example = json!(UNAUTHORIZED_EXAMPLE)),
        (status = 403, description = "Forbidden", body = ErrorResponse, example = json!(FORBIDDEN_EXAMPLE)),
        (status = 500, description = "Internal Server Error", body = ErrorResponse, example = json!(INTERNAL_EXAMPLE))
    )
)]
async fn list_api_keys(
    State(app_state): State<Arc<AppState>>,
    user: User,
    Query(filter): Query<ApiKeyFilter>,
) -> Result<Json<Vec<ApiKeyResponse>>, ApplicationError> {
    user.check_permission(Permission::ReadApiKey)?;

    let api_keys = app_state.services.api_key.list(filter).await?;
    let response: Vec<ApiKeyResponse> = api_keys.into_iter().map(Into::into).collect();

    Ok(response.into())
}

/// Revoke an API Key
///
/// Revokes an API Key by ID. Returns an empty body.
#[utoipa::path(
    delete,
    path = "/{id}",
    tag = "API Keys",
    context_path = CONTEXT_PATH,
    responses(
        (status = 200, description = "Revoked"),
        (status = 400, description = "Bad Request", body = ErrorResponse, example = json!(BAD_REQUEST_EXAMPLE)),
        (status = 401, description = "Unauthorized", body = ErrorResponse, example = json!(UNAUTHORIZED_EXAMPLE)),
        (status = 403, description = "Forbidden", body = ErrorResponse, example = json!(FORBIDDEN_EXAMPLE)),
        (status = 404, description = "Not Found", body = ErrorResponse, example = json!(NOT_FOUND_EXAMPLE)),
        (status = 500, description = "Internal Server Error", body = ErrorResponse, example = json!(INTERNAL_EXAMPLE))
    )
)]
async fn revoke_api_key(
    State(app_state): State<Arc<AppState>>,
    user: User,
    Path(id): Path<Uuid>,
) -> Result<(), ApplicationError> {
    user.check_permission(Permission::WriteApiKey)?;

    app_state.services.api_key.revoke(id).await?;
    Ok(())
}

/// Revoke API Keys
///
/// Revokes all the API Keys given a filter. Returns the number of revoked keys.
#[utoipa::path(
    delete,
    path = "",
    tag = "API Keys",
    context_path = CONTEXT_PATH,
    params(ApiKeyFilter),
    responses(
        (status = 200, description = "Success", body = u64),
        (status = 400, description = "Bad Request", body = ErrorResponse, example = json!(BAD_REQUEST_EXAMPLE)),
        (status = 401, description = "Unauthorized", body = ErrorResponse, example = json!(UNAUTHORIZED_EXAMPLE)),
        (status = 403, description = "Forbidden", body = ErrorResponse, example = json!(FORBIDDEN_EXAMPLE)),
        (status = 500, description = "Internal Server Error", body = ErrorResponse, example = json!(INTERNAL_EXAMPLE))
    )
)]
async fn revoke_api_keys(
    State(app_state): State<Arc<AppState>>,
    user: User,
    Query(query_params): Query<ApiKeyFilter>,
) -> Result<Json<u64>, ApplicationError> {
    user.check_permission(Permission::WriteApiKey)?;

    let n_revoked = app_state.services.api_key.revoke_many(query_params).await?;
    Ok(n_revoked.into())
}
