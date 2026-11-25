use super::{CreateOfferRequest, OfferFilter, WalletOffer};
use crate::{
    application::{
        composition::AppServices,
        errors::{ApplicationError, DataError},
    },
    domains::account::{Permission, User},
    infra::axum::{Json, Path, Query},
};
use axum::{extract::State, routing::get, Router};
use std::sync::Arc;
use swissknife_types::ErrorResponse;
use utoipa::OpenApi;
use uuid::Uuid;
#[derive(OpenApi)]
#[openapi(
    paths(create_offer, list_offers, disable_offer, create_wallet_offer, list_wallet_offers, disable_wallet_offer),
    components(schemas(WalletOffer, CreateOfferRequest, swissknife_types::OfferState, swissknife_types::OfferInvoice)),
    tags((name = "Offers", description = "Wallet-owned BOLT12 receive offers."))
)]
pub struct OfferHandler;
pub fn router() -> Router<Arc<AppServices>> {
    Router::new()
        .route("/v1/offers", get(list_offers).post(create_offer))
        .route("/v1/offers/{id}", axum::routing::delete(disable_offer))
        .route(
            "/v1/me/wallets/{wallet_id}/offers",
            get(list_wallet_offers).post(create_wallet_offer),
        )
        .route(
            "/v1/me/wallets/{wallet_id}/offers/{id}",
            axum::routing::delete(disable_wallet_offer),
        )
}

/// Create BOLT12 offer.
#[utoipa::path(
    post,
    path = "/v1/offers",
    tag = "Offers",
    request_body = CreateOfferRequest,
    responses(
        (status = 200, description = "Success", body = WalletOffer),
        (status = 401, description = "Authentication required", body = ErrorResponse),
        (status = 403, description = "Insufficient permission", body = ErrorResponse),
        (status = 404, description = "Wallet or offer not found", body = ErrorResponse),
        (status = 409, description = "Conflicting request", body = ErrorResponse),
        (status = 422, description = "Unsupported offer request", body = ErrorResponse)
    )
)]
async fn create_offer(
    State(services): State<Arc<AppServices>>,
    user: User,
    Json(request): Json<CreateOfferRequest>,
) -> Result<Json<WalletOffer>, ApplicationError> {
    user.check_permission(Permission::WriteTransaction)?;
    let wallet_id = request
        .wallet_id
        .ok_or_else(|| DataError::Malformed("wallet_id is required".into()))?;
    Ok(Json(services.offer.create(wallet_id, request).await?))
}

/// List BOLT12 offers.
#[utoipa::path(
    get,
    path = "/v1/offers",
    tag = "Offers",
    params(OfferFilter),
    responses(
        (status = 200, description = "Success", body = Vec<WalletOffer>),
        (status = 401, description = "Authentication required", body = ErrorResponse),
        (status = 403, description = "Insufficient permission", body = ErrorResponse),
        (status = 404, description = "Wallet or offer not found", body = ErrorResponse),
        (status = 409, description = "Conflicting request", body = ErrorResponse),
        (status = 422, description = "Unsupported offer request", body = ErrorResponse)
    )
)]
async fn list_offers(
    State(services): State<Arc<AppServices>>,
    user: User,
    Query(filter): Query<OfferFilter>,
) -> Result<Json<Vec<WalletOffer>>, ApplicationError> {
    user.check_permission(Permission::ReadTransaction)?;
    Ok(Json(services.offer.list(filter).await?))
}

/// Disable BOLT12 offer.
#[utoipa::path(
    delete,
    path = "/v1/offers/{id}",
    tag = "Offers",
    responses(
        (status = 200, description = "Success", body = WalletOffer),
        (status = 401, description = "Authentication required", body = ErrorResponse),
        (status = 403, description = "Insufficient permission", body = ErrorResponse),
        (status = 404, description = "Wallet or offer not found", body = ErrorResponse),
        (status = 409, description = "Conflicting request", body = ErrorResponse),
        (status = 422, description = "Unsupported offer request", body = ErrorResponse)
    )
)]
async fn disable_offer(
    State(services): State<Arc<AppServices>>,
    user: User,
    Path(id): Path<Uuid>,
) -> Result<Json<WalletOffer>, ApplicationError> {
    user.check_permission(Permission::WriteTransaction)?;
    Ok(Json(services.offer.disable(id, None).await?))
}

/// Create wallet-owned BOLT12 offer.
#[utoipa::path(
    post,
    path = "/v1/me/wallets/{wallet_id}/offers",
    tag = "Offers",
    request_body = CreateOfferRequest,
    responses(
        (status = 200, description = "Success", body = WalletOffer),
        (status = 401, description = "Authentication required", body = ErrorResponse),
        (status = 403, description = "Insufficient permission", body = ErrorResponse),
        (status = 404, description = "Wallet or offer not found", body = ErrorResponse),
        (status = 409, description = "Conflicting request", body = ErrorResponse),
        (status = 422, description = "Unsupported offer request", body = ErrorResponse)
    )
)]
async fn create_wallet_offer(
    State(services): State<Arc<AppServices>>,
    user: User,
    Path(wallet_id): Path<Uuid>,
    Json(request): Json<CreateOfferRequest>,
) -> Result<Json<WalletOffer>, ApplicationError> {
    services.wallet.verify_ownership(user.account_id, wallet_id).await?;
    Ok(Json(services.offer.create(wallet_id, request).await?))
}

/// List wallet-owned BOLT12 offers.
#[utoipa::path(
    get,
    path = "/v1/me/wallets/{wallet_id}/offers",
    tag = "Offers",
    params(OfferFilter),
    responses(
        (status = 200, description = "Success", body = Vec<WalletOffer>),
        (status = 401, description = "Authentication required", body = ErrorResponse),
        (status = 403, description = "Insufficient permission", body = ErrorResponse),
        (status = 404, description = "Wallet or offer not found", body = ErrorResponse),
        (status = 409, description = "Conflicting request", body = ErrorResponse),
        (status = 422, description = "Unsupported offer request", body = ErrorResponse)
    )
)]
async fn list_wallet_offers(
    State(services): State<Arc<AppServices>>,
    user: User,
    Path(wallet_id): Path<Uuid>,
    Query(mut filter): Query<OfferFilter>,
) -> Result<Json<Vec<WalletOffer>>, ApplicationError> {
    services.wallet.verify_ownership(user.account_id, wallet_id).await?;
    filter.wallet_id = Some(wallet_id);
    Ok(Json(services.offer.list(filter).await?))
}

/// Disable wallet-owned BOLT12 offer.
#[utoipa::path(
    delete,
    path = "/v1/me/wallets/{wallet_id}/offers/{id}",
    tag = "Offers",
    responses(
        (status = 200, description = "Success", body = WalletOffer),
        (status = 401, description = "Authentication required", body = ErrorResponse),
        (status = 403, description = "Insufficient permission", body = ErrorResponse),
        (status = 404, description = "Wallet or offer not found", body = ErrorResponse),
        (status = 409, description = "Conflicting request", body = ErrorResponse),
        (status = 422, description = "Unsupported offer request", body = ErrorResponse)
    )
)]
async fn disable_wallet_offer(
    State(services): State<Arc<AppServices>>,
    user: User,
    Path((wallet_id, id)): Path<(Uuid, Uuid)>,
) -> Result<Json<WalletOffer>, ApplicationError> {
    services.wallet.verify_ownership(user.account_id, wallet_id).await?;
    Ok(Json(services.offer.disable(id, Some(wallet_id)).await?))
}
