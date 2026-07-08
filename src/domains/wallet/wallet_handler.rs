use std::sync::Arc;

use axum::{
    extract::{Path, State},
    routing::{delete, get, post},
    Router,
};
use axum_extra::extract::Query;
use utoipa::OpenApi;
use uuid::Uuid;

use swissknife_types::{CreateWalletRequest, ErrorResponse};

use crate::{
    application::{
        composition::AppServices,
        docs::{
            BAD_REQUEST_EXAMPLE, FORBIDDEN_EXAMPLE, INTERNAL_EXAMPLE, NOT_FOUND_EXAMPLE, UNAUTHORIZED_EXAMPLE,
            UNPROCESSABLE_EXAMPLE,
        },
        errors::ApplicationError,
    },
    domains::user::{Permission, User},
    infra::axum::Json,
};

use super::{Wallet, WalletFilter, WalletOverview};

#[derive(OpenApi)]
#[openapi(
    paths(register_wallet, list_wallets, list_wallet_overviews, get_wallet, delete_wallet, delete_wallets),
    components(schemas(Wallet, WalletOverview, CreateWalletRequest)),
    tags(
        (name = "Wallets", description = "Wallet management endpoints. Require `read:wallet` or `write:wallet` permissions.")
    ),
)]
pub struct WalletHandler;
pub const CONTEXT_PATH: &str = "/v1/wallets";

pub fn router() -> Router<Arc<AppServices>> {
    Router::new()
        .route("/", post(register_wallet))
        .route("/", get(list_wallets))
        .route("/overviews", get(list_wallet_overviews))
        .route("/{id}", get(get_wallet))
        .route("/{id}", delete(delete_wallet))
        .route("/", delete(delete_wallets))
}

/// Register a new wallet
///
/// Returns the account asset wallet.
#[utoipa::path(
    post,
    path = "",
    tag = "Wallets",
    context_path = CONTEXT_PATH,
    request_body = CreateWalletRequest,
    responses(
        (status = 200, description = "Wallet Created", body = Wallet),
        (status = 400, description = "Bad Request", body = ErrorResponse, example = json!(BAD_REQUEST_EXAMPLE)),
        (status = 401, description = "Unauthorized", body = ErrorResponse, example = json!(UNAUTHORIZED_EXAMPLE)),
        (status = 403, description = "Forbidden", body = ErrorResponse, example = json!(FORBIDDEN_EXAMPLE)),
        (status = 422, description = "Unprocessable Entity", body = ErrorResponse, example = json!(UNPROCESSABLE_EXAMPLE)),
        (status = 500, description = "Internal Server Error", body = ErrorResponse, example = json!(INTERNAL_EXAMPLE))
    )
)]
async fn register_wallet(
    State(services): State<Arc<AppServices>>,
    user: User,
    Json(payload): Json<CreateWalletRequest>,
) -> Result<Json<Wallet>, ApplicationError> {
    user.check_permission(Permission::WriteWallet)?;

    let wallet = services.wallet.create(payload.account_id, payload.asset_id).await?;
    Ok(Json(wallet))
}

/// List wallets
///
/// Returns all the wallets without any linked data. Use the wallet ID to get the full wallet details.
#[utoipa::path(
    get,
    path = "",
    tag = "Wallets",
    context_path = CONTEXT_PATH,
    responses(
        (status = 200, description = "Success", body = Vec<Wallet>),
        (status = 400, description = "Bad Request", body = ErrorResponse, example = json!(BAD_REQUEST_EXAMPLE)),
        (status = 401, description = "Unauthorized", body = ErrorResponse, example = json!(UNAUTHORIZED_EXAMPLE)),
        (status = 403, description = "Forbidden", body = ErrorResponse, example = json!(FORBIDDEN_EXAMPLE)),
        (status = 500, description = "Internal Server Error", body = ErrorResponse, example = json!(INTERNAL_EXAMPLE))
    )
)]
async fn list_wallets(
    State(services): State<Arc<AppServices>>,
    user: User,
    Query(filter): Query<WalletFilter>,
) -> Result<Json<Vec<Wallet>>, ApplicationError> {
    user.check_permission(Permission::ReadWallet)?;

    let wallets = services.wallet.list(filter).await?;

    Ok(Json(wallets))
}

/// List wallet overviews
///
/// Returns all the wallet overviews. A wallet overview is a summary of a wallet with the number of payments, invoices and contacts.
#[utoipa::path(
    get,
    path = "/overviews",
    tag = "Wallets",
    context_path = CONTEXT_PATH,
    responses(
        (status = 200, description = "Success", body = Vec<WalletOverview>),
        (status = 400, description = "Bad Request", body = ErrorResponse, example = json!(BAD_REQUEST_EXAMPLE)),
        (status = 401, description = "Unauthorized", body = ErrorResponse, example = json!(UNAUTHORIZED_EXAMPLE)),
        (status = 403, description = "Forbidden", body = ErrorResponse, example = json!(FORBIDDEN_EXAMPLE)),
        (status = 500, description = "Internal Server Error", body = ErrorResponse, example = json!(INTERNAL_EXAMPLE))
    )
)]
async fn list_wallet_overviews(
    State(services): State<Arc<AppServices>>,
    user: User,
) -> Result<Json<Vec<WalletOverview>>, ApplicationError> {
    user.check_permission(Permission::ReadWallet)?;

    let overviews = services.wallet.list_overviews().await?;

    Ok(Json(overviews))
}

/// Find a wallet
///
/// Returns the wallet by its ID.
#[utoipa::path(
    get,
    path = "/{id}",
    tag = "Wallets",
    context_path = CONTEXT_PATH,
    responses(
        (status = 200, description = "Found", body = Wallet),
        (status = 400, description = "Bad Request", body = ErrorResponse, example = json!(BAD_REQUEST_EXAMPLE)),
        (status = 401, description = "Unauthorized", body = ErrorResponse, example = json!(UNAUTHORIZED_EXAMPLE)),
        (status = 403, description = "Forbidden", body = ErrorResponse, example = json!(FORBIDDEN_EXAMPLE)),
        (status = 404, description = "Not Found", body = ErrorResponse, example = json!(NOT_FOUND_EXAMPLE)),
        (status = 500, description = "Internal Server Error", body = ErrorResponse, example = json!(INTERNAL_EXAMPLE))
    )
)]
async fn get_wallet(
    State(services): State<Arc<AppServices>>,
    user: User,
    Path(id): Path<Uuid>,
) -> Result<Json<Wallet>, ApplicationError> {
    user.check_permission(Permission::ReadWallet)?;

    let wallet = services.wallet.get(id).await?;
    Ok(Json(wallet))
}

/// Delete a wallet
///
/// Deletes an wallet by ID. Returns an empty body. Deleting a wallet removes all data related to that wallet.
#[utoipa::path(
    delete,
    path = "/{id}",
    tag = "Wallets",
    context_path = CONTEXT_PATH,
    responses(
        (status = 200, description = "Deleted"),
        (status = 400, description = "Bad Request", body = ErrorResponse, example = json!(BAD_REQUEST_EXAMPLE)),
        (status = 401, description = "Unauthorized", body = ErrorResponse, example = json!(UNAUTHORIZED_EXAMPLE)),
        (status = 403, description = "Forbidden", body = ErrorResponse, example = json!(FORBIDDEN_EXAMPLE)),
        (status = 404, description = "Not Found", body = ErrorResponse, example = json!(NOT_FOUND_EXAMPLE)),
        (status = 500, description = "Internal Server Error", body = ErrorResponse, example = json!(INTERNAL_EXAMPLE))
    )
)]
async fn delete_wallet(
    State(services): State<Arc<AppServices>>,
    user: User,
    Path(id): Path<Uuid>,
) -> Result<(), ApplicationError> {
    user.check_permission(Permission::WriteWallet)?;

    services.wallet.delete(id).await?;
    Ok(())
}

/// Delete wallets
///
/// Deletes all the wallets given a filter. Returns the number of deleted wallets. Deleting a wallet removes all data related to that wallet.
#[utoipa::path(
    delete,
    path = "",
    tag = "Wallets",
    context_path = CONTEXT_PATH,
    params(WalletFilter),
    responses(
        (status = 200, description = "Success", body = u64),
        (status = 400, description = "Bad Request", body = ErrorResponse, example = json!(BAD_REQUEST_EXAMPLE)),
        (status = 401, description = "Unauthorized", body = ErrorResponse, example = json!(UNAUTHORIZED_EXAMPLE)),
        (status = 403, description = "Forbidden", body = ErrorResponse, example = json!(FORBIDDEN_EXAMPLE)),
        (status = 500, description = "Internal Server Error", body = ErrorResponse, example = json!(INTERNAL_EXAMPLE))
    )
)]
async fn delete_wallets(
    State(services): State<Arc<AppServices>>,
    user: User,
    Query(query_params): Query<WalletFilter>,
) -> Result<Json<u64>, ApplicationError> {
    user.check_permission(Permission::WriteWallet)?;

    let n_deleted = services.wallet.delete_many(query_params).await?;
    Ok(n_deleted.into())
}

#[cfg(test)]
mod tests {
    use crate::{application::composition::MockAppServicesBuilder, domains::wallet::Wallet};

    use super::*;

    fn user(permissions: Vec<Permission>) -> User {
        User {
            wallet_id: Uuid::new_v4(),
            permissions,
            ..Default::default()
        }
    }

    mod register_wallet {
        use super::*;

        mod without_the_write_permission {
            use super::*;

            #[tokio::test]
            async fn is_forbidden_and_does_not_call_the_service() {
                let services = MockAppServicesBuilder::new().build();
                let account_id = Uuid::new_v4();
                let asset_id = Uuid::new_v4();

                let result = register_wallet(
                    State(Arc::new(services)),
                    user(vec![]),
                    Json(CreateWalletRequest { account_id, asset_id }),
                )
                .await;

                assert!(matches!(result, Err(ApplicationError::Authorization(_))));
            }
        }

        mod with_the_write_permission {
            use super::*;

            #[tokio::test]
            async fn delegates_to_the_service() {
                let account_id = Uuid::new_v4();
                let asset_id = Uuid::new_v4();
                let mut builder = MockAppServicesBuilder::new();
                builder
                    .wallet
                    .expect_create()
                    .withf(move |account, asset| *account == account_id && *asset == asset_id)
                    .times(1)
                    .returning(|_, _| Ok(Wallet::default()));

                let result = register_wallet(
                    State(Arc::new(builder.build())),
                    user(vec![Permission::WriteWallet]),
                    Json(CreateWalletRequest { account_id, asset_id }),
                )
                .await;

                assert!(result.is_ok());
            }
        }
    }

    mod get_wallet {
        use super::*;

        mod without_the_read_permission {
            use super::*;

            #[tokio::test]
            async fn is_forbidden() {
                let services = MockAppServicesBuilder::new().build();

                let result = get_wallet(State(Arc::new(services)), user(vec![]), Path(Uuid::new_v4())).await;

                assert!(matches!(result, Err(ApplicationError::Authorization(_))));
            }
        }
    }
}
