use std::sync::Arc;

use axum::{extract::State, routing::get, Json, Router};
use breez_sdk_core::{LspInformation, NodeState, Payment, ServiceHealthCheckResponse};

use crate::{
    application::{
        dtos::{LightningPaymentResponse, SendPaymentRequest},
        errors::ApplicationError,
    },
    domains::users::entities::AuthUser,
    infra::app::AppState,
};

pub struct LightningNodeHandler;

impl LightningNodeHandler {
    pub fn routes() -> Router<Arc<AppState>> {
        Router::new()
            .route("/info", get(Self::node_info))
            .route("/lsp-info", get(Self::lsp_info))
            .route("/list-payments", get(Self::list_payments))
            .route("/send-payment", get(Self::send_payment))
            .route("/health", get(Self::health_check))
    }

    async fn node_info(
        State(app_state): State<Arc<AppState>>,
        user: AuthUser,
    ) -> Result<Json<NodeState>, ApplicationError> {
        let node_info = app_state.lightning.node_info(user).await?;

        Ok(node_info.into())
    }

    async fn lsp_info(
        State(app_state): State<Arc<AppState>>,
        user: AuthUser,
    ) -> Result<Json<LspInformation>, ApplicationError> {
        let lsp_info = app_state.lightning.lsp_info(user).await?;

        Ok(lsp_info.into())
    }

    async fn list_payments(
        State(app_state): State<Arc<AppState>>,
        user: AuthUser,
    ) -> Result<Json<Vec<Payment>>, ApplicationError> {
        let payments = app_state.lightning.list_node_payments(user).await?;

        Ok(payments.into())
    }

    async fn send_payment(
        State(app_state): State<Arc<AppState>>,
        user: AuthUser,
        Json(payload): Json<SendPaymentRequest>,
    ) -> Result<Json<LightningPaymentResponse>, ApplicationError> {
        let payment = app_state
            .lightning
            .send_payment(user, payload.input, payload.amount_msat, payload.comment)
            .await?;

        Ok(Json(payment.into()))
    }

    async fn health_check(
        State(app_state): State<Arc<AppState>>,
        user: AuthUser,
    ) -> Result<Json<ServiceHealthCheckResponse>, ApplicationError> {
        let health = app_state.lightning.health_check(user).await?;

        Ok(health.into())
    }
}
