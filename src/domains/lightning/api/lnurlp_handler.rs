use std::sync::Arc;

use axum::{
    extract::{Path, Query, State},
    routing::get,
    Json, Router,
};

use crate::{
    application::{dtos::LNUrlpInvoiceQueryParams, errors::ApplicationError},
    domains::lightning::entities::{LnURLPayRequest, LnUrlCallbackResponse},
    infra::app::AppState,
};

pub struct LnURLpHandler;

impl LnURLpHandler {
    pub fn well_known_route() -> Router<Arc<AppState>> {
        Router::new().route("/:username", get(Self::well_known_lnurlp))
    }

    pub fn callback_route() -> Router<Arc<AppState>> {
        Router::new().route("/:username/callback", get(Self::callback))
    }

    async fn well_known_lnurlp(
        Path(username): Path<String>,
        State(app_state): State<Arc<AppState>>,
    ) -> Result<Json<LnURLPayRequest>, ApplicationError> {
        let lnurlp = app_state.services.lnurl.lnurlp(username).await?;
        Ok(lnurlp.into())
    }

    async fn callback(
        Path(username): Path<String>,
        Query(query_params): Query<LNUrlpInvoiceQueryParams>,
        State(app_state): State<Arc<AppState>>,
    ) -> Result<Json<LnUrlCallbackResponse>, ApplicationError> {
        let callback = app_state
            .services
            .lnurl
            .lnurlp_callback(username, query_params.amount, query_params.comment)
            .await?;
        Ok(callback.into())
    }
}
