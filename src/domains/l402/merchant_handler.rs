use super::MerchantCatalog;
use crate::{
    application::{composition::AppServices, errors::ApplicationError},
    domains::account::User,
    infra::axum::{Json, Path},
};
use axum::{
    extract::State,
    http::StatusCode,
    routing::{get, put},
    Extension, Router,
};
use std::sync::Arc;
use swissknife_types::{MerchantOrigin, MerchantSaleTerms, WalletMerchantCatalog, WalletMerchantRoute};
use utoipa::OpenApi;
use uuid::Uuid;

#[derive(OpenApi)]
#[openapi(
    paths(list_wallet_merchant_routes, set_wallet_merchant_route),
    components(schemas(MerchantOrigin, MerchantSaleTerms, WalletMerchantCatalog, WalletMerchantRoute))
)]
pub struct WalletMerchantHandler;

pub fn router(catalog: Arc<MerchantCatalog>) -> Router<Arc<AppServices>> {
    Router::new()
        .route(
            "/v1/me/wallets/{wallet_id}/l402/routes",
            get(list_wallet_merchant_routes),
        )
        .route(
            "/v1/me/wallets/{wallet_id}/l402/routes/{name}",
            put(set_wallet_merchant_route),
        )
        .layer(Extension(catalog))
}
/// List this wallet’s approved origins and its configured sales.
#[utoipa::path(get,path="/v1/me/wallets/{wallet_id}/l402/routes",tag="L402",security(("jwt"=[]),("api_key"=[])),
params(("wallet_id"=Uuid,Path)),responses((status=200,body=WalletMerchantCatalog),(status=401),(status=404)))]
async fn list_wallet_merchant_routes(
    State(services): State<Arc<AppServices>>,
    Extension(catalog): Extension<Arc<MerchantCatalog>>,
    owner: User,
    Path(wallet): Path<Uuid>,
) -> Result<Json<WalletMerchantCatalog>, ApplicationError> {
    services.wallet.verify_ownership(owner.account_id, wallet).await?;
    Ok(Json(catalog.list(wallet).await?))
}
/// Publish, reprice or pause a wallet route for an operator-approved origin.
#[utoipa::path(put,path="/v1/me/wallets/{wallet_id}/l402/routes/{name}",tag="L402",security(("jwt"=[]),("api_key"=[])),
params(("wallet_id"=Uuid,Path),("name"=String,Path)),request_body=MerchantSaleTerms,responses((status=204),(status=401),(status=404),(status=409),(status=422)))]
async fn set_wallet_merchant_route(
    State(services): State<Arc<AppServices>>,
    Extension(catalog): Extension<Arc<MerchantCatalog>>,
    owner: User,
    Path((wallet, name)): Path<(Uuid, String)>,
    Json(terms): Json<MerchantSaleTerms>,
) -> Result<StatusCode, ApplicationError> {
    services.wallet.verify_ownership(owner.account_id, wallet).await?;
    catalog.set(wallet, &name, terms).await?;
    Ok(StatusCode::NO_CONTENT)
}
