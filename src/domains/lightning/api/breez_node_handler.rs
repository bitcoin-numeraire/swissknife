use std::sync::Arc;

use axum::{
    body::Body,
    extract::State,
    http::header,
    response::{IntoResponse, Response},
    routing::{get, post},
    Json, Router,
};
use breez_sdk_core::{LspInformation, NodeState, ReverseSwapInfo, ServiceHealthCheckResponse};

use crate::{
    application::{
        dtos::{
            CheckMessageRequest, ConnectLSPRequest, RedeemOnchainRequest,
            SendOnchainPaymentRequest, SignMessageRequest,
        },
        errors::{ApplicationError, LightningError},
    },
    domains::users::entities::{AuthUser, Permission},
    infra::{app::AppState, lightning::LnClient},
};

pub struct BreezNodeHandler;

impl BreezNodeHandler {
    pub fn routes() -> Router<Arc<AppState>> {
        Router::new()
            .route("/info", get(Self::node_info))
            .route("/lsp-info", get(Self::lsp_info))
            .route("/lsps", get(Self::list_lsps))
            .route("/close-channels", post(Self::close_lsp_channels))
            .route("/connect-lsp", post(Self::connect_lsp))
            .route("/swap", post(Self::swap))
            .route("/redeem", post(Self::redeem))
            .route("/sign-message", post(Self::sign_message))
            .route("/check-message", post(Self::check_message))
            .route("/sync", post(Self::sync))
            .route("/backup", get(Self::backup))
            .route("/health", get(Self::health_check))
    }

    async fn node_info(
        State(app_state): State<Arc<AppState>>,
        user: AuthUser,
    ) -> Result<Json<NodeState>, ApplicationError> {
        user.check_permission(Permission::ReadLnNode)?;

        let client = app_state.ln_node_client.as_breez_client()?;
        let node_info = client.node_info()?;

        Ok(node_info.into())
    }

    async fn lsp_info(
        State(app_state): State<Arc<AppState>>,
        user: AuthUser,
    ) -> Result<Json<LspInformation>, ApplicationError> {
        user.check_permission(Permission::ReadLnNode)?;

        let client = app_state.ln_node_client.as_breez_client()?;
        let lsp_info = client.lsp_info().await?;

        Ok(lsp_info.into())
    }

    async fn list_lsps(
        State(app_state): State<Arc<AppState>>,
        user: AuthUser,
    ) -> Result<Json<Vec<LspInformation>>, ApplicationError> {
        user.check_permission(Permission::ReadLnNode)?;

        let client = app_state.ln_node_client.as_breez_client()?;
        let lsps = client.list_lsps().await?;

        Ok(lsps.into())
    }

    async fn close_lsp_channels(
        State(app_state): State<Arc<AppState>>,
        user: AuthUser,
    ) -> Result<Json<Vec<String>>, ApplicationError> {
        user.check_permission(Permission::WriteLnNode)?;

        let client = app_state.ln_node_client.as_breez_client()?;
        let tx_ids = client.close_lsp_channels().await?;

        Ok(tx_ids.into())
    }

    async fn connect_lsp(
        State(app_state): State<Arc<AppState>>,
        user: AuthUser,
        Json(payload): Json<ConnectLSPRequest>,
    ) -> Result<(), ApplicationError> {
        user.check_permission(Permission::WriteLnNode)?;

        let client = app_state.ln_node_client.as_breez_client()?;
        client.connect_lsp(payload.lsp_id).await?;

        Ok(())
    }

    // TODO: Move to pay and parse the input to check if it's a BTC address instead of own endpoint
    async fn swap(
        State(app_state): State<Arc<AppState>>,
        user: AuthUser,
        Json(payload): Json<SendOnchainPaymentRequest>,
    ) -> Result<Json<ReverseSwapInfo>, ApplicationError> {
        user.check_permission(Permission::WriteLnNode)?;

        let client = app_state.ln_node_client.as_breez_client()?;
        let payment_info = client
            .pay_onchain(
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
        user.check_permission(Permission::WriteLnNode)?;

        let client = app_state.ln_node_client.as_breez_client()?;
        let txid = client
            .redeem_onchain(payload.to_address, payload.feerate)
            .await?;

        Ok(txid)
    }

    async fn sign_message(
        State(app_state): State<Arc<AppState>>,
        user: AuthUser,
        Json(payload): Json<SignMessageRequest>,
    ) -> Result<String, ApplicationError> {
        user.check_permission(Permission::WriteLnNode)?;

        let client = app_state.ln_node_client.as_breez_client()?;
        let signature = client.sign_message(payload.message).await?;

        Ok(signature)
    }

    async fn check_message(
        State(app_state): State<Arc<AppState>>,
        user: AuthUser,
        Json(payload): Json<CheckMessageRequest>,
    ) -> Result<String, ApplicationError> {
        user.check_permission(Permission::WriteLnNode)?;

        let client = app_state.ln_node_client.as_breez_client()?;
        let is_valid = client
            .check_message(payload.message, payload.pubkey, payload.signature)
            .await?;

        Ok(is_valid.to_string())
    }

    async fn sync(
        State(app_state): State<Arc<AppState>>,
        user: AuthUser,
    ) -> Result<(), ApplicationError> {
        user.check_permission(Permission::WriteLnNode)?;

        let client = app_state.ln_node_client.as_breez_client()?;
        client.sync().await?;

        Ok(())
    }

    async fn backup(
        State(app_state): State<Arc<AppState>>,
        user: AuthUser,
    ) -> Result<Response, ApplicationError> {
        user.check_permission(Permission::ReadLnNode)?;

        let client = app_state.ln_node_client.as_breez_client()?;
        let data = client.backup()?;

        return match data {
            Some(data) => {
                let filename = "channels_backup.txt";
                let body = Body::from(data.join("\n").into_bytes());

                let headers = [
                    (header::CONTENT_TYPE, "text/plain"),
                    (
                        header::CONTENT_DISPOSITION,
                        &format!("attachment; filename=\"{}\"", filename),
                    ),
                ];

                Ok((headers, body).into_response())
            }
            None => Err(LightningError::Backup("No backup data found".to_string()))?,
        };
    }

    async fn health_check(
        State(app_state): State<Arc<AppState>>,
        user: AuthUser,
    ) -> Result<Json<ServiceHealthCheckResponse>, ApplicationError> {
        user.check_permission(Permission::ReadLnNode)?;

        let client = app_state.ln_node_client.as_breez_client()?;
        let health = client.health().await?;

        Ok(health.into())
    }
}
