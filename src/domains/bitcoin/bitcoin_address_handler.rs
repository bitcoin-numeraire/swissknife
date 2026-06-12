use std::sync::Arc;

use axum::{
    extract::State,
    routing::{delete, get, post},
    Router,
};
use utoipa::OpenApi;
use uuid::Uuid;

use crate::{
    application::{
        docs::{
            BAD_REQUEST_EXAMPLE, FORBIDDEN_EXAMPLE, INTERNAL_EXAMPLE, NOT_FOUND_EXAMPLE, UNAUTHORIZED_EXAMPLE,
            UNPROCESSABLE_EXAMPLE,
        },
        dtos::{ErrorResponse, NewBtcAddressRequest},
        entities::AppServices,
        errors::ApplicationError,
    },
    domains::{
        bitcoin::{BtcAddress, BtcAddressFilter, BtcAddressType, BtcNetwork},
        user::{Permission, User},
    },
    infra::axum::{Json, Path, Query},
};

#[derive(OpenApi)]
#[openapi(
    paths(generate_btc_address, list_btc_addresses, get_btc_address, delete_btc_address, delete_btc_addresses),
    components(schemas(NewBtcAddressRequest, BtcAddress, BtcNetwork, BtcAddressType)),
    tags(
        (name = "Bitcoin Addresses", description = "Bitcoin Address management endpoints. Require `read:btc_address` or `write:btc_address` permissions.")
    ),
)]
pub struct BtcAddressHandler;
pub const CONTEXT_PATH: &str = "/v1/bitcoin/addresses";

pub fn router() -> Router<Arc<AppServices>> {
    Router::new()
        .route("/", post(generate_btc_address))
        .route("/", get(list_btc_addresses))
        .route("/{id}", get(get_btc_address))
        .route("/{id}", delete(delete_btc_address))
        .route("/", delete(delete_btc_addresses))
}

/// Generate a new Bitcoin address
///
/// Returns the generated Bitcoin address for the given user
#[utoipa::path(
    post,
    path = "",
    tag = "Bitcoin Addresses",
    context_path = CONTEXT_PATH,
    request_body = NewBtcAddressRequest,
    responses(
        (status = 200, description = "Bitcoin Address Created", body = BtcAddress),
        (status = 400, description = "Bad Request", body = ErrorResponse, example = json!(BAD_REQUEST_EXAMPLE)),
        (status = 401, description = "Unauthorized", body = ErrorResponse, example = json!(UNAUTHORIZED_EXAMPLE)),
        (status = 403, description = "Forbidden", body = ErrorResponse, example = json!(FORBIDDEN_EXAMPLE)),
        (status = 422, description = "Unprocessable Entity", body = ErrorResponse, example = json!(UNPROCESSABLE_EXAMPLE)),
        (status = 500, description = "Internal Server Error", body = ErrorResponse, example = json!(INTERNAL_EXAMPLE))
    )
)]
async fn generate_btc_address(
    State(services): State<Arc<AppServices>>,
    user: User,
    Json(payload): Json<NewBtcAddressRequest>,
) -> Result<Json<BtcAddress>, ApplicationError> {
    user.check_permission(Permission::WriteBtcAddress)?;

    let address = services
        .bitcoin
        .new_deposit_address(payload.wallet_id.unwrap_or(user.wallet_id), payload.address_type)
        .await?;
    Ok(Json(address))
}

/// Find a Bitcoin address
///
/// Returns the Bitcoin address by its ID.
#[utoipa::path(
    get,
    path = "/{id}",
    tag = "Bitcoin Addresses",
    context_path = CONTEXT_PATH,
    responses(
        (status = 200, description = "Found", body = BtcAddress),
        (status = 400, description = "Bad Request", body = ErrorResponse, example = json!(BAD_REQUEST_EXAMPLE)),
        (status = 401, description = "Unauthorized", body = ErrorResponse, example = json!(UNAUTHORIZED_EXAMPLE)),
        (status = 403, description = "Forbidden", body = ErrorResponse, example = json!(FORBIDDEN_EXAMPLE)),
        (status = 404, description = "Not Found", body = ErrorResponse, example = json!(NOT_FOUND_EXAMPLE)),
        (status = 500, description = "Internal Server Error", body = ErrorResponse, example = json!(INTERNAL_EXAMPLE))
    )
)]
async fn get_btc_address(
    State(services): State<Arc<AppServices>>,
    user: User,
    Path(id): Path<Uuid>,
) -> Result<Json<BtcAddress>, ApplicationError> {
    user.check_permission(Permission::ReadBtcAddress)?;

    let address = services.bitcoin.get_address(id).await?;
    Ok(Json(address))
}

/// List Bitcoin addresses
///
/// Returns all the Bitcoin addresses given a filter
#[utoipa::path(
    get,
    path = "",
    tag = "Bitcoin Addresses",
    context_path = CONTEXT_PATH,
    params(BtcAddressFilter),
    responses(
        (status = 200, description = "Success", body = Vec<BtcAddress>),
        (status = 400, description = "Bad Request", body = ErrorResponse, example = json!(BAD_REQUEST_EXAMPLE)),
        (status = 401, description = "Unauthorized", body = ErrorResponse, example = json!(UNAUTHORIZED_EXAMPLE)),
        (status = 403, description = "Forbidden", body = ErrorResponse, example = json!(FORBIDDEN_EXAMPLE)),
        (status = 500, description = "Internal Server Error", body = ErrorResponse, example = json!(INTERNAL_EXAMPLE))
    )
)]
async fn list_btc_addresses(
    State(services): State<Arc<AppServices>>,
    user: User,
    Query(filter): Query<BtcAddressFilter>,
) -> Result<Json<Vec<BtcAddress>>, ApplicationError> {
    user.check_permission(Permission::ReadBtcAddress)?;

    let addresses = services.bitcoin.list_addresses(filter).await?;

    Ok(Json(addresses))
}

/// Delete a Bitcoin address
///
/// Deletes an Bitcoin address by ID. Returns an empty body. Deleting a Bitcoin address has an effect on the user balance
#[utoipa::path(
    delete,
    path = "/{id}",
    tag = "Bitcoin Addresses",
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
async fn delete_btc_address(
    State(services): State<Arc<AppServices>>,
    user: User,
    Path(id): Path<Uuid>,
) -> Result<(), ApplicationError> {
    user.check_permission(Permission::WriteBtcAddress)?;

    services.bitcoin.delete_address(id).await?;
    Ok(())
}

/// Delete Bitcoin addresses
///
/// Deletes all the Bitcoin addresses given a filter. Returns the number of deleted addresses. Deleting an address can have an effect on the user balance
#[utoipa::path(
    delete,
    path = "",
    tag = "Bitcoin Addresses",
    context_path = CONTEXT_PATH,
    params(BtcAddressFilter),
    responses(
        (status = 200, description = "Success", body = u64),
        (status = 400, description = "Bad Request", body = ErrorResponse, example = json!(BAD_REQUEST_EXAMPLE)),
        (status = 401, description = "Unauthorized", body = ErrorResponse, example = json!(UNAUTHORIZED_EXAMPLE)),
        (status = 403, description = "Forbidden", body = ErrorResponse, example = json!(FORBIDDEN_EXAMPLE)),
        (status = 500, description = "Internal Server Error", body = ErrorResponse, example = json!(INTERNAL_EXAMPLE))
    )
)]
async fn delete_btc_addresses(
    State(services): State<Arc<AppServices>>,
    user: User,
    Query(query_params): Query<BtcAddressFilter>,
) -> Result<Json<u64>, ApplicationError> {
    user.check_permission(Permission::WriteBtcAddress)?;

    let n_deleted = services.bitcoin.delete_many_addresses(query_params).await?;
    Ok(n_deleted.into())
}

#[cfg(test)]
mod tests {
    use chrono::Utc;

    use crate::{application::entities::MockAppServicesBuilder, domains::bitcoin::BtcAddress};

    use super::*;

    fn user(permissions: Vec<Permission>) -> User {
        User {
            id: "alice".to_string(),
            wallet_id: Uuid::new_v4(),
            permissions,
        }
    }

    fn btc_address(wallet_id: Uuid) -> BtcAddress {
        BtcAddress {
            id: Uuid::new_v4(),
            wallet_id,
            address: "bcrt1qexample".to_string(),
            used: false,
            address_type: BtcAddressType::P2wpkh,
            created_at: Utc::now(),
            updated_at: None,
        }
    }

    mod generate_btc_address {
        use super::*;

        mod without_the_write_permission {
            use super::*;

            #[tokio::test]
            async fn is_forbidden_and_does_not_call_the_service() {
                let services = MockAppServicesBuilder::new().build();

                let payload = NewBtcAddressRequest {
                    wallet_id: None,
                    address_type: None,
                };

                let result = generate_btc_address(State(Arc::new(services)), user(vec![]), Json(payload)).await;

                assert!(matches!(result, Err(ApplicationError::Authorization(_))));
            }
        }

        mod when_wallet_id_is_omitted {
            use super::*;

            #[tokio::test]
            async fn defaults_to_the_authenticated_users_wallet() {
                let caller = user(vec![Permission::WriteBtcAddress]);
                let expected_wallet = caller.wallet_id;

                let mut builder = MockAppServicesBuilder::new();
                builder
                    .bitcoin
                    .expect_new_deposit_address()
                    .withf(move |wallet_id, _| *wallet_id == expected_wallet)
                    .times(1)
                    .returning(|wallet_id, _| Ok(btc_address(wallet_id)));

                let payload = NewBtcAddressRequest {
                    wallet_id: None,
                    address_type: None,
                };

                let result = generate_btc_address(State(Arc::new(builder.build())), caller, Json(payload)).await;

                assert!(result.is_ok());
            }
        }
    }

    mod get_btc_address {
        use super::*;

        mod without_the_read_permission {
            use super::*;

            #[tokio::test]
            async fn is_forbidden() {
                let services = MockAppServicesBuilder::new().build();

                let result = get_btc_address(State(Arc::new(services)), user(vec![]), Path(Uuid::new_v4())).await;

                assert!(matches!(result, Err(ApplicationError::Authorization(_))));
            }
        }
    }
}
