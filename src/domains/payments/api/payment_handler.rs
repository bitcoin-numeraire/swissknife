use std::sync::Arc;

use axum::{
    extract::{Path, State},
    routing::{delete, get, post},
    Json, Router,
};
use axum_extra::extract::Query;
use utoipa::OpenApi;
use uuid::Uuid;

use crate::{
    application::{dtos::SendPaymentRequest, errors::ApplicationError},
    domains::{
        payments::entities::{Payment, PaymentFilter},
        users::entities::{AuthUser, Permission},
    },
    infra::app::AppState,
};

#[derive(OpenApi)]
#[openapi(paths(pay), components(schemas(Payment)))]
pub struct PaymentHandler;

pub fn router() -> Router<Arc<AppState>> {
    Router::new()
        .route("/", post(pay))
        .route("/", get(list))
        .route("/:id", get(get_one))
        .route("/:id", delete(delete_one))
        .route("/", delete(delete_many))
}

#[utoipa::path(
    post,
    path = "/",
    responses(
        (status = 200, description = "Send a payment")
    )
)]
async fn pay(
    State(app_state): State<Arc<AppState>>,
    user: AuthUser,
    Json(payload): Json<SendPaymentRequest>,
) -> Result<Json<Payment>, ApplicationError> {
    user.check_permission(Permission::WriteLnTransaction)?;

    let payment = app_state.services.payment.pay(payload).await?;
    Ok(payment.into())
}

async fn get_one(
    State(app_state): State<Arc<AppState>>,
    user: AuthUser,
    Path(id): Path<Uuid>,
) -> Result<Json<Payment>, ApplicationError> {
    user.check_permission(Permission::ReadLnTransaction)?;

    let payment = app_state.services.payment.get(id).await?;
    Ok(payment.into())
}

async fn list(
    State(app_state): State<Arc<AppState>>,
    user: AuthUser,
    Query(query_params): Query<PaymentFilter>,
) -> Result<Json<Vec<Payment>>, ApplicationError> {
    user.check_permission(Permission::ReadLnTransaction)?;

    let payments = app_state.services.payment.list(query_params).await?;

    let response: Vec<Payment> = payments.into_iter().map(Into::into).collect();

    Ok(response.into())
}

async fn delete_one(
    State(app_state): State<Arc<AppState>>,
    user: AuthUser,
    Path(id): Path<Uuid>,
) -> Result<(), ApplicationError> {
    user.check_permission(Permission::WriteLnTransaction)?;

    app_state.services.payment.delete(id).await?;
    Ok(())
}

async fn delete_many(
    State(app_state): State<Arc<AppState>>,
    user: AuthUser,
    Query(query_params): Query<PaymentFilter>,
) -> Result<Json<u64>, ApplicationError> {
    user.check_permission(Permission::WriteLnTransaction)?;

    let n_deleted = app_state.services.payment.delete_many(query_params).await?;
    Ok(n_deleted.into())
}
