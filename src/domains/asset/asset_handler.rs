use std::sync::Arc;

use axum::{extract::State, routing::get, Router};
use swissknife_types::ErrorResponse;
use utoipa::OpenApi;

use super::Asset;
use crate::{
    application::{
        composition::AppServices,
        docs::{INTERNAL_EXAMPLE, UNAUTHORIZED_EXAMPLE},
        errors::ApplicationError,
    },
    domains::account::User,
    infra::axum::Json,
};

#[derive(OpenApi)]
#[openapi(paths(list_assets), tags((name = "Assets", description = "Installation asset catalog.")))]
pub struct AssetHandler;
pub const CONTEXT_PATH: &str = "/v1/assets";

pub fn router() -> Router<Arc<AppServices>> {
    Router::new().route("/", get(list_assets))
}

/// List catalog assets
///
/// Any authenticated account can discover assets, including assets with no wallets.
/// Results are ordered by code, protocol, network, and asset reference.
#[utoipa::path(
    get,
    path = "",
    tag = "Assets",
    context_path = CONTEXT_PATH,
    responses(
        (status = 200, description = "Catalog assets", body = Vec<Asset>, example = json!([{
            "id": "00000000-0000-4000-8000-000000000001", "code": "BTC",
            "name": "Bitcoin", "protocol": "bitcoin", "network": "Bitcoin",
            "asset_ref": "native", "display_ticker": "BTC", "decimals": 11,
            "created_at": "2026-07-04T00:00:00Z"
        }])),
        (status = 401, description = "Unauthorized", body = ErrorResponse, example = json!(UNAUTHORIZED_EXAMPLE)),
        (status = 500, description = "Database unavailable", body = ErrorResponse, example = json!(INTERNAL_EXAMPLE))
    )
)]
async fn list_assets(
    State(services): State<Arc<AppServices>>,
    _user: User,
) -> Result<Json<Vec<Asset>>, ApplicationError> {
    Ok(Json(services.asset.list().await?))
}
