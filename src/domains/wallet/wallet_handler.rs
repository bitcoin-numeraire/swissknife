use std::sync::Arc;

use axum::{extract::State, routing::get, Json, Router};
use utoipa::OpenApi;

use crate::{
    application::{
        docs::{BAD_REQUEST_EXAMPLE, FORBIDDEN_EXAMPLE, INTERNAL_EXAMPLE, UNAUTHORIZED_EXAMPLE},
        dtos::WalletResponse,
        errors::ApplicationError,
    },
    domains::user::{Permission, User},
    infra::app::AppState,
};

#[derive(OpenApi)]
#[openapi(
    paths(list_wallets),
    tags(
        (name = "Wallets", description = "Wallet management endpoints. Require `read:wallet` or `write:wallet` permissions.")
    ),
)]
pub struct WalletHandler;
pub const CONTEXT_PATH: &str = "/v1/wallets";

pub fn router() -> Router<Arc<AppState>> {
    Router::new().route("/", get(list_wallets))
}

/// List wallets
///
/// Returns all the wallets
#[utoipa::path(
    get,
    path = "",
    tag = "Wallets",
    context_path = CONTEXT_PATH,
    responses(
        (status = 200, description = "Success", body = Vec<WalletResponse>),
        (status = 400, description = "Bad Request", body = ErrorResponse, example = json!(BAD_REQUEST_EXAMPLE)),
        (status = 401, description = "Unauthorized", body = ErrorResponse, example = json!(UNAUTHORIZED_EXAMPLE)),
        (status = 403, description = "Forbidden", body = ErrorResponse, example = json!(FORBIDDEN_EXAMPLE)),
        (status = 500, description = "Internal Server Error", body = ErrorResponse, example = json!(INTERNAL_EXAMPLE))
    )
)]
async fn list_wallets(
    State(app_state): State<Arc<AppState>>,
    user: User,
) -> Result<Json<Vec<WalletResponse>>, ApplicationError> {
    user.check_permission(Permission::ReadWallet)?;

    let wallets = app_state.services.wallet.list().await?;
    let response: Vec<WalletResponse> = wallets.into_iter().map(Into::into).collect();

    Ok(response.into())
}
