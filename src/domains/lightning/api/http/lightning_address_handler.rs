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
            PaginationQueryParams, RegisterLightningAddressRequest,
        },
        errors::ApplicationError,
    },
    domains::{lightning::entities::LNURLPayRequest, users::entities::AuthUser},
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
    ) -> Result<Json<LNURLPayRequest>, ApplicationError> {
        let lnurlp = app_state.lightning.generate_lnurlp(username).await?;

        Ok(lnurlp.into())
    }

    async fn invoice(
        Path(username): Path<String>,
        Query(query_params): Query<LNUrlpInvoiceQueryParams>,
        State(app_state): State<Arc<AppState>>,
    ) -> Result<Json<LNUrlpInvoiceResponse>, ApplicationError> {
        let invoice = app_state
            .lightning
            .generate_invoice(username, query_params.amount, query_params.description)
            .await?;

        Ok(LNUrlpInvoiceResponse::new(invoice.bolt11).into())
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
