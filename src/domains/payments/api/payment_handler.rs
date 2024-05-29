use std::sync::Arc;

use axum::{
    extract::{Path, Query, State},
    routing::{delete, get, post},
    Json, Router,
};
use uuid::Uuid;

use crate::{
    application::{dtos::SendPaymentRequest, errors::ApplicationError},
    domains::{
        payments::entities::{Payment, PaymentFilter},
        users::entities::{AuthUser, Permission},
    },
    infra::app::AppState,
};

pub struct PaymentHandler;

impl PaymentHandler {
    pub fn routes() -> Router<Arc<AppState>> {
        Router::new()
            .route("/", post(Self::pay))
            .route("/", get(Self::list))
            .route("/:id", get(Self::get))
            .route("/:id", delete(Self::delete))
            .route("/", delete(Self::delete_many))
    }

    async fn pay(
        State(app_state): State<Arc<AppState>>,
        user: AuthUser,
        Json(payload): Json<SendPaymentRequest>,
    ) -> Result<Json<Payment>, ApplicationError> {
        user.check_permission(Permission::WriteLightningTransaction)?;

        let payment = app_state.services.payment.pay(payload).await?;
        Ok(Json(payment.into()))
    }

    async fn get(
        State(app_state): State<Arc<AppState>>,
        user: AuthUser,
        Path(id): Path<Uuid>,
    ) -> Result<Json<Payment>, ApplicationError> {
        user.check_permission(Permission::ReadLightningTransaction)?;

        let lightning_address = app_state.services.payment.get(id).await?;
        Ok(Json(lightning_address.into()))
    }

    async fn list(
        State(app_state): State<Arc<AppState>>,
        user: AuthUser,
        Query(query_params): Query<PaymentFilter>,
    ) -> Result<Json<Vec<Payment>>, ApplicationError> {
        user.check_permission(Permission::ReadLightningTransaction)?;

        let lightning_payments = app_state.services.payment.list(query_params).await?;

        let response: Vec<Payment> = lightning_payments.into_iter().map(Into::into).collect();

        Ok(response.into())
    }

    async fn delete(
        State(app_state): State<Arc<AppState>>,
        user: AuthUser,
        Path(id): Path<Uuid>,
    ) -> Result<(), ApplicationError> {
        user.check_permission(Permission::WriteLightningTransaction)?;

        app_state.services.payment.delete(id).await?;
        Ok(())
    }

    async fn delete_many(
        State(app_state): State<Arc<AppState>>,
        user: AuthUser,
        Query(query_params): Query<PaymentFilter>,
    ) -> Result<Json<u64>, ApplicationError> {
        user.check_permission(Permission::WriteLightningTransaction)?;

        let n_deleted = app_state.services.payment.delete_many(query_params).await?;
        Ok(n_deleted.into())
    }
}
