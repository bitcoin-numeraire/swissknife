use std::sync::Arc;

use axum::{extract::State, routing::get, Json, Router};
use breez_sdk_core::ServiceHealthCheckResponse;

use crate::{
    application::errors::ApplicationError,
    domains::users::entities::{AuthUser, Permission},
    infra::{app::AppState, lightning::LnClient},
};

pub struct ClnNodeHandler;

impl ClnNodeHandler {
    pub fn routes() -> Router<Arc<AppState>> {
        Router::new().route("/health", get(Self::health_check))
    }

    async fn health_check(
        State(app_state): State<Arc<AppState>>,
        user: AuthUser,
    ) -> Result<Json<ServiceHealthCheckResponse>, ApplicationError> {
        user.check_permission(Permission::ReadLnNode)?;

        let client = app_state.ln_node_client.as_cln_client()?;
        let health = client.health().await?;

        Ok(health.into())
    }
}
