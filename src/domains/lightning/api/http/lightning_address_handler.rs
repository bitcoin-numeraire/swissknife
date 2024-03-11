use std::sync::Arc;

use axum::{
    extract::{Path, Query, State},
    routing::{get, post},
    Json, Router,
};

use crate::{
    adapters::app::AppState,
    application::{
        dtos::{
            LNUrlpInvoiceQueryParams, LNUrlpInvoiceResponse, LightningAddressResponse,
            LightningWellKnownResponse, PaginationQueryParams, RegisterLightningAddressRequest,
            SuccessAction,
        },
        errors::ApplicationError,
    },
    domains::users::entities::AuthUser,
};

pub struct LightningAddressHandler;

impl LightningAddressHandler {
    pub fn well_known_routes() -> Router<Arc<AppState>> {
        Router::new().route("/:username", get(Self::well_known_lnurlp))
    }

    pub fn addresses_routes() -> Router<Arc<AppState>> {
        Router::new()
            .route("/", get(Self::list))
            .route("/:username", get(Self::get))
            .route("/:username/invoice", get(Self::invoice))
            .route("/register", post(Self::register))
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
        Query(query_params): Query<LNUrlpInvoiceQueryParams>,
        State(app_state): State<Arc<AppState>>,
    ) -> Result<Json<LNUrlpInvoiceResponse>, ApplicationError> {
        let invoice = app_state
            .lightning
            .generate_invoice(username, query_params.amount, query_params.comment)
            .await?;

        let response = LNUrlpInvoiceResponse {
            pr: invoice.bolt11,
            success_action: Some(SuccessAction {
                tag: "message".to_string(),
                message: Some("Thanks for the sats!".to_string()),
            }),
            disposable: None,
            routes: vec![],
        };

        Ok(response.into())
    }

    async fn register(
        State(app_state): State<Arc<AppState>>,
        user: AuthUser,
        Json(payload): Json<RegisterLightningAddressRequest>,
    ) -> Result<Json<LightningAddressResponse>, ApplicationError> {
        let lightning_address = app_state
            .lightning
            .register_lightning_address(user, payload.username)
            .await?;

        Ok(Json(lightning_address.into()))
    }

    async fn get(
        State(app_state): State<Arc<AppState>>,
        user: AuthUser,
        Path(username): Path<String>,
    ) -> Result<Json<LightningAddressResponse>, ApplicationError> {
        let lightning_address = app_state
            .lightning
            .get_lightning_address(user, username)
            .await?;

        Ok(Json(lightning_address.into()))
    }

    async fn list(
        State(app_state): State<Arc<AppState>>,
        user: AuthUser,
        Query(query_params): Query<PaginationQueryParams>,
    ) -> Result<Json<Vec<LightningAddressResponse>>, ApplicationError> {
        let limit = query_params.limit.unwrap_or(100);
        let offset = query_params.offset.unwrap_or(0);

        let lightning_addresses = app_state
            .lightning
            .list_lightning_addresses(user, limit, offset)
            .await?;

        let response: Vec<LightningAddressResponse> =
            lightning_addresses.into_iter().map(Into::into).collect();

        Ok(response.into())
    }
}
