use std::sync::Arc;

use axum::{
    extract::State,
    routing::{get, post},
    Json, Router,
};
use breez_sdk_core::{
    LspInformation, NodeState, Payment, ReverseSwapInfo, ServiceHealthCheckResponse,
};

use crate::{
    application::{
        dtos::{RedeemOnchainRequest, SendOnchainPaymentRequest},
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
            .route("/lsps", get(Self::list_lsps))
            .route("/payments", get(Self::list_payments))
            .route("/close-channels", post(Self::close_lsp_channels))
            .route("/swap", post(Self::swap))
            .route("/redeem", post(Self::redeem))
            .route("/health", get(Self::health_check))
    }

    async fn node_info(
        State(app_state): State<Arc<AppState>>,
        user: AuthUser,
    ) -> Result<Json<NodeState>, ApplicationError> {
        let node_info = app_state.services.lightning.node_info(user).await?;

        Ok(node_info.into())
    }

    async fn lsp_info(
        State(app_state): State<Arc<AppState>>,
        user: AuthUser,
    ) -> Result<Json<LspInformation>, ApplicationError> {
        let lsp_info = app_state.services.lightning.lsp_info(user).await?;

        Ok(lsp_info.into())
    }

    async fn list_payments(
        State(app_state): State<Arc<AppState>>,
        user: AuthUser,
    ) -> Result<Json<Vec<Payment>>, ApplicationError> {
        let payments = app_state
            .services
            .lightning
            .list_node_payments(user)
            .await?;

        Ok(payments.into())
    }

    async fn list_lsps(
        State(app_state): State<Arc<AppState>>,
        user: AuthUser,
    ) -> Result<Json<Vec<LspInformation>>, ApplicationError> {
        let lsps = app_state.services.lightning.list_lsps(user).await?;

        Ok(lsps.into())
    }

    async fn close_lsp_channels(
        State(app_state): State<Arc<AppState>>,
        user: AuthUser,
    ) -> Result<Json<Vec<String>>, ApplicationError> {
        let tx_ids = app_state
            .services
            .lightning
            .close_lsp_channels(user)
            .await?;

        Ok(tx_ids.into())
    }

    // TODO: Move to pay and parse the input to check if it's a BTC address instead of own endpoint
    async fn swap(
        State(app_state): State<Arc<AppState>>,
        user: AuthUser,
        Json(payload): Json<SendOnchainPaymentRequest>,
    ) -> Result<Json<ReverseSwapInfo>, ApplicationError> {
        let payment_info = app_state
            .services
            .lightning
            .pay_onchain(
                user,
                payload.amount_msat,
                payload.recipient_address,
                payload.feerate,
            )
            .await?;

        Ok(payment_info.into())
    }

    async fn redeem(
        State(app_state): State<Arc<AppState>>,
        user: AuthUser,
        Json(payload): Json<RedeemOnchainRequest>,
    ) -> Result<String, ApplicationError> {
        let txid = app_state
            .services
            .lightning
            .redeem(user, payload.to_address, payload.feerate)
            .await?;

        Ok(txid)
    }

    async fn health_check(
        State(app_state): State<Arc<AppState>>,
        user: AuthUser,
    ) -> Result<Json<ServiceHealthCheckResponse>, ApplicationError> {
        let health = app_state.services.lightning.health_check(user).await?;

        Ok(health.into())
    }
}
