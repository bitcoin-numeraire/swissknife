use std::sync::Arc;

use axum::{
    extract::State,
    http::StatusCode,
    response::IntoResponse,
    routing::{get, post},
    Router,
};
use utoipa::OpenApi;

use crate::{
    application::{docs::INTERNAL_EXAMPLE, dtos::ErrorResponse, errors::ApplicationError},
    infra::{app::AppState, axum::Json},
};

use super::{HealthCheck, HealthStatus, SetupInfo, VersionInfo};

#[derive(OpenApi)]
#[openapi(
    paths(readiness_check, health_check, version_check, setup_check, mark_welcome_complete),
    components(schemas(HealthCheck, HealthStatus, VersionInfo, SetupInfo)),
    tags(
        (name = "System", description = "System related endpoints")
    )
)]
pub struct SystemHandler;
pub const CONTEXT_PATH: &str = "/v1/system";

pub fn router() -> Router<Arc<AppState>> {
    Router::new()
        .route("/health", get(health_check))
        .route("/ready", get(readiness_check))
        .route("/version", get(version_check))
        .route("/setup", get(setup_check))
        .route("/mark-welcome-complete", post(mark_welcome_complete))
}

/// Readiness Check
///
/// Returns successfully if the server is reachable.
#[utoipa::path(
    get,
    path = "/ready",
    tag = "System",
    context_path = CONTEXT_PATH,
    responses(
        (status = 204, description = "No Content on success"),
    )
)]
async fn readiness_check() -> impl IntoResponse {
    StatusCode::NO_CONTENT
}

/// Health Check
///
/// Returns the health of the system fine-grained by dependency.
#[utoipa::path(
    get,
    path = "/health",
    tag = "System",
    context_path = CONTEXT_PATH,
    responses(
        (status = 200, description = "OK", body = HealthCheck),
        (status = 503, description = "Service Unavailable", body = HealthCheck)
    )
)]
async fn health_check(State(app_state): State<Arc<AppState>>) -> impl IntoResponse {
    let health_check = app_state.services.system.health_check().await;

    let status_code = if health_check.is_healthy {
        StatusCode::OK
    } else {
        StatusCode::SERVICE_UNAVAILABLE
    };

    (status_code, Json(health_check))
}

/// Version Information
///
/// Returns the current version and build time of the system.
#[utoipa::path(
    get,
    path = "/version",
    tag = "System",
    context_path = CONTEXT_PATH,
    responses(
        (status = 200, description = "OK", body = VersionInfo)

    )
)]
async fn version_check(State(app_state): State<Arc<AppState>>) -> Json<VersionInfo> {
    app_state.services.system.version().into()
}

/// Setup Status Check
///
/// Returns whether the application setup is complete.
#[utoipa::path(
    get,
    path = "/setup",
    tag = "System",
    context_path = CONTEXT_PATH,
    responses(
        (status = 200, description = "OK", body = SetupInfo),
        (status = 500, description = "Internal Server Error", body = ErrorResponse, example = json!(INTERNAL_EXAMPLE))
    )
)]
async fn setup_check(
    State(app_state): State<Arc<AppState>>,
) -> Result<Json<SetupInfo>, ApplicationError> {
    let info = app_state.services.system.setup_check().await?;
    Ok(info.into())
}

/// Marks the welcome flow as completed
///
/// Returns whether the application welcome flow is complete.
#[utoipa::path(
    post,
    path = "/mark-welcome-complete",
    tag = "System",
    context_path = CONTEXT_PATH,
    responses(
        (status = 204, description = "No Content on success"),
        (status = 500, description = "Internal Server Error", body = ErrorResponse, example = json!(INTERNAL_EXAMPLE))
    )
)]
async fn mark_welcome_complete(
    State(app_state): State<Arc<AppState>>,
) -> Result<impl IntoResponse, ApplicationError> {
    app_state.services.system.mark_welcome_complete().await?;
    Ok(StatusCode::NO_CONTENT)
}
