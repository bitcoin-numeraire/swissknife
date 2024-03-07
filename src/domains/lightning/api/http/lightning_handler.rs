use std::sync::Arc;

use axum::{
    extract::{Path, Query, State},
    routing::{get, post},
    Json, Router,
};
use breez_sdk_core::{LspInformation, NodeState, Payment};

use crate::{
    adapters::app::AppState,
    application::{
        dtos::{
            LightningAddressResponse, LightningInvoiceQueryParams, LightningInvoiceResponse,
            LightningWellKnownResponse, RegisterLightningAddressRequest, SendPaymentRequest,
            SuccessAction,
        },
        errors::ApplicationError,
    },
    domains::users::entities::AuthUser,
};

pub struct LightningHandler;

impl LightningHandler {
    pub fn well_known_routes() -> Router<Arc<AppState>> {
        Router::new()
            .route("/:username", get(Self::well_known_lnurlp))
            .route("/:username/callback", get(Self::invoice))
    }

    pub fn addresses_routes() -> Router<Arc<AppState>> {
        Router::new().route("/register", post(Self::register_lightning_address))
    }

    pub fn node_routes() -> Router<Arc<AppState>> {
        Router::new()
            .route("/info", get(Self::node_info))
            .route("/lsp-info", get(Self::lsp_info))
            .route("/list-payments", get(Self::list_payments))
            .route("/send-payment", get(Self::send_payment))
    }

    async fn well_known_lnurlp(
        Path(username): Path<String>,
        State(app_state): State<Arc<AppState>>,
    ) -> Result<Json<LightningWellKnownResponse>, ApplicationError> {
        let lnurlp = app_state.lightning.generate_lnurlp(username).await?;

        let response = LightningWellKnownResponse {
            callback: lnurlp.callback,
            max_sendable: lnurlp.max_sendable,
            min_sendable: lnurlp.min_sendable,
            metadata: lnurlp.metadata,
            comment_allowed: lnurlp.comment_allowed,
            withdraw_link: lnurlp.withdraw_link,
            tag: lnurlp.tag,
        };

        Ok(response.into())
    }

    async fn invoice(
        Path(username): Path<String>,
        Query(query_params): Query<LightningInvoiceQueryParams>,
        State(app_state): State<Arc<AppState>>,
    ) -> Result<Json<LightningInvoiceResponse>, ApplicationError> {
        let invoice = app_state
            .lightning
            .generate_invoice(username, query_params.amount)
            .await?;

        let response = LightningInvoiceResponse {
            pr: invoice,
            success_action: Some(SuccessAction {
                tag: "message".to_string(),
                message: Some("Thanks for the sats!".to_string()),
            }),
            disposable: None,
            routes: vec![],
        };

        Ok(response.into())
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
        let payments = app_state.lightning.list_payments(user).await?;

        Ok(payments.into())
    }

    async fn register_lightning_address(
        State(app_state): State<Arc<AppState>>,
        user: AuthUser,
        Json(payload): Json<RegisterLightningAddressRequest>,
    ) -> Result<Json<LightningAddressResponse>, ApplicationError> {
        let lightning_address = app_state
            .lightning
            .register_lightning_address(user, payload.username)
            .await?;

        let response = LightningAddressResponse {
            id: lightning_address.id,
            user_id: lightning_address.user_id,
            username: lightning_address.username,
            active: lightning_address.active,
            created_at: lightning_address.created_at,
            updated_at: lightning_address.updated_at,
            deleted_at: lightning_address.deleted_at,
        };

        Ok(response.into())
    }

    async fn send_payment(
        State(app_state): State<Arc<AppState>>,
        user: AuthUser,
        Json(payload): Json<SendPaymentRequest>,
    ) -> Result<Json<Payment>, ApplicationError> {
        let payment = app_state
            .lightning
            .send_bolt11_payment(user, payload.bolt11, payload.amount_msat)
            .await?;

        Ok(payment.into())
    }
}
