use std::sync::Arc;

use axum::{
    extract::{Path, Query, State},
    routing::{delete, get, post},
    Json, Router,
};
use uuid::Uuid;

use crate::{
    application::{
        dtos::{LightningPaymentFilter, LightningPaymentResponse, SendPaymentRequest},
        errors::ApplicationError,
    },
    domains::users::entities::{AuthUser, Permission},
    infra::app::AppState,
};

pub struct LightningPaymentHandler;

impl LightningPaymentHandler {
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
    ) -> Result<Json<LightningPaymentResponse>, ApplicationError> {
        user.check_permission(Permission::WriteLightningTransaction)?;

        let payment = app_state.lightning.pay(payload).await?;
        Ok(Json(payment.into()))
    }

    async fn get(
        State(app_state): State<Arc<AppState>>,
        user: AuthUser,
        Path(id): Path<Uuid>,
    ) -> Result<Json<LightningPaymentResponse>, ApplicationError> {
        user.check_permission(Permission::ReadLightningTransaction)?;

        let lightning_address = app_state.lightning.get_payment(id).await?;
        Ok(Json(lightning_address.into()))
    }

    async fn list(
        State(app_state): State<Arc<AppState>>,
        user: AuthUser,
        Query(query_params): Query<LightningPaymentFilter>,
    ) -> Result<Json<Vec<LightningPaymentResponse>>, ApplicationError> {
        user.check_permission(Permission::ReadLightningTransaction)?;

        let lightning_payments = app_state.lightning.list_payments(query_params).await?;

        let response: Vec<LightningPaymentResponse> =
            lightning_payments.into_iter().map(Into::into).collect();

        Ok(response.into())
    }

    async fn delete(
        State(app_state): State<Arc<AppState>>,
        user: AuthUser,
        Path(id): Path<Uuid>,
    ) -> Result<(), ApplicationError> {
        user.check_permission(Permission::WriteLightningTransaction)?;

        app_state.lightning.delete_payment(id).await?;
        Ok(())
    }

    async fn delete_many(
        State(app_state): State<Arc<AppState>>,
        user: AuthUser,
        Query(query_params): Query<LightningPaymentFilter>,
    ) -> Result<Json<u64>, ApplicationError> {
        user.check_permission(Permission::WriteLightningTransaction)?;

        let n_deleted = app_state.lightning.delete_payments(query_params).await?;
        Ok(n_deleted.into())
    }
}
