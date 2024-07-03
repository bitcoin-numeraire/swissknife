use std::sync::Arc;

use axum::{extract::State, http::StatusCode, response::IntoResponse, routing::get, Json, Router};

use crate::{
    application::errors::ApplicationError,
    domains::system::entities::{HealthCheck, VersionInfo},
    infra::app::AppState,
};

pub struct SystemHandler;

impl SystemHandler {
    pub fn routes() -> Router<Arc<AppState>> {
        Router::new()
            .route("/health", get(Self::health_check))
            .route("/ready", get(Self::readiness_check))
            .route("/version", get(Self::version_check))
    }

    async fn readiness_check() -> impl IntoResponse {
        StatusCode::OK
    }

    async fn health_check(
        State(app_state): State<Arc<AppState>>,
    ) -> Result<Json<HealthCheck>, ApplicationError> {
        let health_check = app_state.services.system.health_check().await;
        Ok(health_check.into())
    }

    async fn version_check(
        State(app_state): State<Arc<AppState>>,
    ) -> Result<Json<VersionInfo>, ApplicationError> {
        let version = app_state.services.system.version();
        Ok(version.into())
    }
}
