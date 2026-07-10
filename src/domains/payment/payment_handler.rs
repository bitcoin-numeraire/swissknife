use std::sync::Arc;

use axum::{
    extract::State,
    routing::{delete, get, post},
    Router,
};
use axum_extra::extract::Query;
use utoipa::OpenApi;
use uuid::Uuid;

use swissknife_types::{ErrorResponse, SendPaymentRequest};

use crate::{
    application::{
        composition::AppServices,
        docs::{
            BAD_REQUEST_EXAMPLE, FORBIDDEN_EXAMPLE, INTERNAL_EXAMPLE, NOT_FOUND_EXAMPLE, UNAUTHORIZED_EXAMPLE,
            UNPROCESSABLE_EXAMPLE,
        },
        errors::{ApplicationError, DataError},
    },
    domains::{
        lnurl::LnUrlSuccessAction,
        user::{Permission, User},
    },
    infra::axum::{Json, Path},
};

use super::{BtcPayment, InternalPayment, LnPayment, Payment, PaymentFilter, PaymentStatus};

#[derive(OpenApi)]
#[openapi(
    paths(pay, get_payment, list_payments, delete_payment, delete_payments),
    components(schemas(
        Payment,
        LnPayment,
        BtcPayment,
        InternalPayment,
        SendPaymentRequest,
        PaymentStatus,
        LnUrlSuccessAction
    )),
    tags(
        (name = "Payments", description = "Payment management endpoints. Require `read:transaction` or `write:transaction` permissions.")
    )
)]
pub struct PaymentHandler;
pub const CONTEXT_PATH: &str = "/v1/payments";

pub fn router() -> Router<Arc<AppServices>> {
    Router::new()
        .route("/", post(pay))
        .route("/", get(list_payments))
        .route("/{id}", get(get_payment))
        .route("/{id}", delete(delete_payment))
        .route("/", delete(delete_payments))
}

/// Send a payment
///
/// Pay for a LN invoice, LNURL, LN Address, On-chain or internally to an other user on the same instance. Returns the payment details.
#[utoipa::path(
    post,
    path = "",
    tag = "Payments",
    context_path = CONTEXT_PATH,
    request_body = SendPaymentRequest,
    responses(
        (status = 200, description = "Payment Sent", body = Payment),
        (status = 400, description = "Bad Request", body = ErrorResponse, example = json!(BAD_REQUEST_EXAMPLE)),
        (status = 401, description = "Unauthorized", body = ErrorResponse, example = json!(UNAUTHORIZED_EXAMPLE)),
        (status = 403, description = "Forbidden", body = ErrorResponse, example = json!(FORBIDDEN_EXAMPLE)),
        (status = 422, description = "Unprocessable Entity", body = ErrorResponse, example = json!(UNPROCESSABLE_EXAMPLE)),
        (status = 500, description = "Internal Server Error", body = ErrorResponse, example = json!(INTERNAL_EXAMPLE))
    )
)]
async fn pay(
    State(services): State<Arc<AppServices>>,
    user: User,
    Json(payload): Json<SendPaymentRequest>,
) -> Result<Json<Payment>, ApplicationError> {
    user.check_permission(Permission::WriteTransaction)?;
    let wallet_id = payload
        .wallet_id
        .ok_or_else(|| DataError::Malformed("wallet_id is required.".to_string()))?;

    let payment = services
        .payment
        .pay(payload.input, payload.amount_msat, payload.comment, wallet_id)
        .await?;

    Ok(Json(payment))
}

/// Find a payment
///
/// Returns the payment by its ID.
#[utoipa::path(
    get,
    path = "/{id}",
    tag = "Payments",
    context_path = CONTEXT_PATH,
    responses(
        (status = 200, description = "Found", body = Payment),
        (status = 400, description = "Bad Request", body = ErrorResponse, example = json!(BAD_REQUEST_EXAMPLE)),
        (status = 401, description = "Unauthorized", body = ErrorResponse, example = json!(UNAUTHORIZED_EXAMPLE)),
        (status = 403, description = "Forbidden", body = ErrorResponse, example = json!(FORBIDDEN_EXAMPLE)),
        (status = 404, description = "Not Found", body = ErrorResponse, example = json!(NOT_FOUND_EXAMPLE)),
        (status = 500, description = "Internal Server Error", body = ErrorResponse, example = json!(INTERNAL_EXAMPLE))
    )
)]
async fn get_payment(
    State(services): State<Arc<AppServices>>,
    user: User,
    Path(id): Path<Uuid>,
) -> Result<Json<Payment>, ApplicationError> {
    user.check_permission(Permission::ReadTransaction)?;

    let payment = services.payment.get(id).await?;
    Ok(Json(payment))
}

/// List payments
///
/// Returns all the payments given a filter
#[utoipa::path(
    get,
    path = "",
    tag = "Payments",
    context_path = CONTEXT_PATH,
    params(PaymentFilter),
    responses(
        (status = 200, description = "Success", body = Vec<Payment>),
        (status = 400, description = "Bad Request", body = ErrorResponse, example = json!(BAD_REQUEST_EXAMPLE)),
        (status = 401, description = "Unauthorized", body = ErrorResponse, example = json!(UNAUTHORIZED_EXAMPLE)),
        (status = 403, description = "Forbidden", body = ErrorResponse, example = json!(FORBIDDEN_EXAMPLE)),
        (status = 500, description = "Internal Server Error", body = ErrorResponse, example = json!(INTERNAL_EXAMPLE))
    )
)]
async fn list_payments(
    State(services): State<Arc<AppServices>>,
    user: User,
    Query(query_params): Query<PaymentFilter>,
) -> Result<Json<Vec<Payment>>, ApplicationError> {
    user.check_permission(Permission::ReadTransaction)?;

    let payments = services.payment.list(query_params).await?;

    Ok(Json(payments))
}

/// Delete a payment
///
/// Deletes a payment by ID. Returns an empty body. Deleting a payment has an effect on the user balance
#[utoipa::path(
    delete,
    path = "/{id}",
    tag = "Payments",
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
async fn delete_payment(
    State(services): State<Arc<AppServices>>,
    user: User,
    Path(id): Path<Uuid>,
) -> Result<(), ApplicationError> {
    user.check_permission(Permission::WriteTransaction)?;

    services.payment.delete(id).await?;
    Ok(())
}

/// Delete payments
///
/// Deletes all the payments given a filter. Returns the number of deleted payments. Deleting a payment can have an effect on the user balance
#[utoipa::path(
    delete,
    path = "",
    tag = "Payments",
    context_path = CONTEXT_PATH,
    params(PaymentFilter),
    responses(
        (status = 200, description = "Success", body = u64),
        (status = 400, description = "Bad Request", body = ErrorResponse, example = json!(BAD_REQUEST_EXAMPLE)),
        (status = 401, description = "Unauthorized", body = ErrorResponse, example = json!(UNAUTHORIZED_EXAMPLE)),
        (status = 403, description = "Forbidden", body = ErrorResponse, example = json!(FORBIDDEN_EXAMPLE)),
        (status = 500, description = "Internal Server Error", body = ErrorResponse, example = json!(INTERNAL_EXAMPLE))
    )
)]
async fn delete_payments(
    State(services): State<Arc<AppServices>>,
    user: User,
    Query(query_params): Query<PaymentFilter>,
) -> Result<Json<u64>, ApplicationError> {
    user.check_permission(Permission::WriteTransaction)?;

    let n_deleted = services.payment.delete_many(query_params).await?;
    Ok(n_deleted.into())
}

#[cfg(test)]
mod tests {
    use crate::{application::composition::MockAppServicesBuilder, domains::payment::Payment};

    use super::*;

    fn user(permissions: Vec<Permission>) -> User {
        User {
            permissions,
            ..Default::default()
        }
    }

    fn send_request(wallet_id: Uuid) -> SendPaymentRequest {
        SendPaymentRequest {
            wallet_id: Some(wallet_id),
            input: "bob@numeraire.tech".to_string(),
            amount_msat: Some(1_000),
            comment: None,
        }
    }

    mod pay {
        use super::*;

        mod without_the_write_permission {
            use super::*;

            #[tokio::test]
            async fn is_forbidden_and_does_not_call_the_service() {
                // No expect_pay installed: any call to the use case panics.
                let services = MockAppServicesBuilder::new().build();

                let result = pay(
                    State(Arc::new(services)),
                    user(vec![]),
                    Json(send_request(Uuid::new_v4())),
                )
                .await;

                assert!(matches!(result, Err(ApplicationError::Authorization(_))));
            }
        }

        mod when_wallet_id_is_provided {
            use super::*;

            #[tokio::test]
            async fn uses_the_explicit_wallet_id() {
                let explicit = Uuid::new_v4();

                let mut builder = MockAppServicesBuilder::new();
                builder
                    .payment
                    .expect_pay()
                    .withf(move |_, _, _, wallet_id| *wallet_id == explicit)
                    .times(1)
                    .returning(|_, _, _, _| Ok(Payment::default()));

                let result = pay(
                    State(Arc::new(builder.build())),
                    user(vec![Permission::WriteTransaction]),
                    Json(send_request(explicit)),
                )
                .await;

                assert!(result.is_ok());
            }
        }
    }

    mod get_payment {
        use super::*;

        mod without_the_read_permission {
            use super::*;

            #[tokio::test]
            async fn is_forbidden() {
                let services = MockAppServicesBuilder::new().build();

                let result = get_payment(State(Arc::new(services)), user(vec![]), Path(Uuid::new_v4())).await;

                assert!(matches!(result, Err(ApplicationError::Authorization(_))));
            }
        }

        mod with_the_read_permission {
            use super::*;

            #[tokio::test]
            async fn returns_the_payment() {
                let id = Uuid::new_v4();

                let mut builder = MockAppServicesBuilder::new();
                builder
                    .payment
                    .expect_get()
                    .withf(move |queried| *queried == id)
                    .times(1)
                    .returning(|_| Ok(Payment::default()));

                let result = get_payment(
                    State(Arc::new(builder.build())),
                    user(vec![Permission::ReadTransaction]),
                    Path(id),
                )
                .await;

                assert!(result.is_ok());
            }
        }
    }

    mod delete_payment {
        use super::*;

        mod without_the_write_permission {
            use super::*;

            #[tokio::test]
            async fn is_forbidden() {
                let services = MockAppServicesBuilder::new().build();

                let err = delete_payment(State(Arc::new(services)), user(vec![]), Path(Uuid::new_v4()))
                    .await
                    .unwrap_err();

                assert!(matches!(err, ApplicationError::Authorization(_)));
            }
        }
    }
}
