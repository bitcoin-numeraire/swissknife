use std::sync::Arc;

use axum::{
    extract::{Path, Query, State},
    routing::get,
    Json, Router,
};

use crate::{
    application::{
        dtos::{LNUrlpInvoiceQueryParams, LNUrlpInvoiceResponse},
        errors::ApplicationError,
    },
    domains::lightning::entities::LNURLPayRequest,
    infra::app::AppState,
};

pub struct LNURLpHandler;

impl LNURLpHandler {
    pub fn routes() -> Router<Arc<AppState>> {
        Router::new()
            .route("/:username", get(Self::well_known_lnurlp))
            .route("/:username/callback", get(Self::invoice))
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
            .generate_invoice(
                username,
                query_params.amount,
                query_params.comment.unwrap_or_default(),
            )
            .await?;

        Ok(Json(invoice.into()))
    }
}
