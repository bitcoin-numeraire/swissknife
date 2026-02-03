use std::sync::Arc;

use axum::{
    extract::State,
    routing::{delete, get, post},
    Router,
};
use axum_extra::extract::Query;
use utoipa::OpenApi;
use uuid::Uuid;

use crate::{
    application::{
        docs::{
            BAD_REQUEST_EXAMPLE, FORBIDDEN_EXAMPLE, INTERNAL_EXAMPLE, NOT_FOUND_EXAMPLE, UNAUTHORIZED_EXAMPLE,
            UNPROCESSABLE_EXAMPLE,
        },
        dtos::{
            BtcPaymentResponse, ErrorResponse, InternalPaymentResponse, LnPaymentResponse, PaymentResponse,
            SendPaymentRequest,
        },
        entities::AppServices,
        errors::ApplicationError,
    },
    domains::{
        lnurl::LnUrlSuccessAction,
        user::{Permission, User},
    },
    infra::axum::{Json, Path},
};

use super::{PaymentFilter, PaymentStatus};

#[derive(OpenApi)]
#[openapi(
    paths(pay, get_payment, list_payments, delete_payment, delete_payments),
    components(schemas(
        PaymentResponse,
        LnPaymentResponse,
        BtcPaymentResponse,
        InternalPaymentResponse,
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
        .route("/:id", get(get_payment))
        .route("/:id", delete(delete_payment))
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
        (status = 200, description = "Payment Sent", body = PaymentResponse),
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
) -> Result<Json<PaymentResponse>, ApplicationError> {
    user.check_permission(Permission::WriteLnTransaction)?;

    let payment = services
        .payment
        .pay(
            payload.input,
            payload.amount_msat,
            payload.comment,
            payload.wallet_id.unwrap_or(user.wallet_id),
        )
        .await?;

    Ok(Json(payment.into()))
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
        (status = 200, description = "Found", body = PaymentResponse),
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
) -> Result<Json<PaymentResponse>, ApplicationError> {
    user.check_permission(Permission::ReadLnTransaction)?;

    let payment = services.payment.get(id).await?;
    Ok(Json(payment.into()))
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
        (status = 200, description = "Success", body = Vec<PaymentResponse>),
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
) -> Result<Json<Vec<PaymentResponse>>, ApplicationError> {
    user.check_permission(Permission::ReadLnTransaction)?;

    let payments = services.payment.list(query_params).await?;
    let response: Vec<PaymentResponse> = payments.into_iter().map(Into::into).collect();

    Ok(response.into())
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
    user.check_permission(Permission::WriteLnTransaction)?;

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
    user.check_permission(Permission::WriteLnTransaction)?;

    let n_deleted = services.payment.delete_many(query_params).await?;
    Ok(n_deleted.into())
}
