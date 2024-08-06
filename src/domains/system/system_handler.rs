use std::sync::Arc;

use axum::{extract::State, http::StatusCode, response::IntoResponse, routing::get, Json, Router};
use utoipa::OpenApi;

use crate::{application::errors::ApplicationError, infra::app::AppState};

use super::{HealthCheck, HealthStatus, VersionInfo};

#[derive(OpenApi)]
#[openapi(
    paths(readiness_check, health_check, version_check),
    components(schemas(HealthCheck, HealthStatus, VersionInfo)),
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
        (status = 200, description = "OK")
    )
)]
async fn readiness_check() -> impl IntoResponse {
    StatusCode::OK
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
async fn version_check(
    State(app_state): State<Arc<AppState>>,
) -> Result<Json<VersionInfo>, ApplicationError> {
    let version = app_state.services.system.version();
    Ok(version.into())
}
