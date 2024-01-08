use std::sync::Arc;

use axum::{
    extract::{Path, Query, State},
    routing::{get, post},
    Json, Router,
};
use breez_sdk_core::{NodeState, Payment};

use crate::{
    adapters::app::AppState,
    application::{
        dtos::{
            LightningAddressResponse, LightningInvoiceQueryParams, LightningInvoiceResponse,
            LightningWellKnownResponse, RegisterLightningAddressRequest, SuccessAction,
        },
        errors::LightningError,
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
            .route("/list-payments", get(Self::list_payments))
    }

    async fn well_known_lnurlp(
        Path(username): Path<String>,
        State(app_state): State<Arc<AppState>>,
    ) -> Result<Json<LightningWellKnownResponse>, LightningError> {
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
    ) -> Result<Json<LightningInvoiceResponse>, LightningError> {
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
    ) -> Result<Json<NodeState>, LightningError> {
        println!("user: {:?}", user);
        let node_info = app_state.lightning.node_info(user.sub).await?;

        Ok(node_info.into())
    }

    async fn list_payments(
        State(app_state): State<Arc<AppState>>,
        user: AuthUser,
    ) -> Result<Json<Vec<Payment>>, LightningError> {
        let payments = app_state.lightning.list_payments(user.sub).await?;

        Ok(payments.into())
    }

    async fn register_lightning_address(
        State(app_state): State<Arc<AppState>>,
        user: AuthUser,
        Json(payload): Json<RegisterLightningAddressRequest>,
    ) -> Result<Json<LightningAddressResponse>, LightningError> {
        let lightning_address = app_state
            .lightning
            .register_lightning_address(user.sub, payload.username)
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
}
