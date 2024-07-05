use std::sync::Arc;

use axum::{
    extract::{Path, Query, State},
    routing::get,
    Json, Router,
};
use utoipa::OpenApi;

use crate::{
    application::{dtos::LNUrlpInvoiceQueryParams, errors::ApplicationError},
    domains::lightning::entities::{LnURLPayRequest, LnUrlCallbackResponse},
    infra::app::AppState,
};

#[derive(OpenApi)]
#[openapi(
    paths(well_known_lnurlp, callback),
    components(schemas(LnURLPayRequest, LnUrlCallbackResponse, ApplicationError))
)]
pub struct LnURLpHandler;

pub fn well_known_router() -> Router<Arc<AppState>> {
    Router::new().route("/:username", get(well_known_lnurlp))
}

pub fn callback_router() -> Router<Arc<AppState>> {
    Router::new().route("/:username/callback", get(callback))
}

#[utoipa::path(
    get,
    path = "/:username",
    responses(
        (status = 200, description = "Get LNURLp of the given user")
    )
)]
async fn well_known_lnurlp(
    Path(username): Path<String>,
    State(app_state): State<Arc<AppState>>,
) -> Result<Json<LnURLPayRequest>, ApplicationError> {
    let lnurlp = app_state.services.lnurl.lnurlp(username).await?;
    Ok(lnurlp.into())
}

#[utoipa::path(
    get,
    path = "/:username/callback",
    responses(
        (status = 200, description = "LNURL callback to get invoice of the given user")
    )
)]
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
