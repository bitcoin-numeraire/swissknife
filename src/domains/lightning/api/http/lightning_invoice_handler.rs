use std::sync::Arc;

use axum::{
    extract::{Path, Query, State},
    routing::{delete, get, post},
    Json, Router,
};
use uuid::Uuid;

use crate::{
    application::{
        dtos::{LightningInvoiceFilter, LightningInvoiceResponse, NewInvoiceRequest},
        errors::ApplicationError,
    },
    domains::users::entities::{AuthUser, Permission},
    infra::app::AppState,
};

pub struct LightningInvoiceHandler;

impl LightningInvoiceHandler {
    pub fn routes() -> Router<Arc<AppState>> {
        Router::new()
            .route("/", post(Self::generate))
            .route("/", get(Self::list))
            .route("/:id", get(Self::get))
            .route("/:id", delete(Self::delete))
            .route("/", delete(Self::delete_many))
    }

    async fn generate(
        State(app_state): State<Arc<AppState>>,
        user: AuthUser,
        Json(payload): Json<NewInvoiceRequest>,
    ) -> Result<Json<LightningInvoiceResponse>, ApplicationError> {
        user.check_permission(Permission::WriteLightningTransaction)?;

        let lightning_address = app_state
            .lightning
            .generate_invoice(
                payload.user_id,
                payload.amount_msat,
                payload.description,
                payload.expiry,
            )
            .await?;
        Ok(Json(lightning_address.into()))
    }

    async fn get(
        State(app_state): State<Arc<AppState>>,
        user: AuthUser,
        Path(id): Path<Uuid>,
    ) -> Result<Json<LightningInvoiceResponse>, ApplicationError> {
        user.check_permission(Permission::ReadLightningTransaction)?;

        let lightning_address = app_state.lightning.get_invoice(id).await?;
        Ok(Json(lightning_address.into()))
    }

    async fn list(
        State(app_state): State<Arc<AppState>>,
        user: AuthUser,
        Query(query_params): Query<LightningInvoiceFilter>,
    ) -> Result<Json<Vec<LightningInvoiceResponse>>, ApplicationError> {
        user.check_permission(Permission::ReadLightningTransaction)?;

        let lightning_invoices = app_state.lightning.list_invoices(query_params).await?;

        let response: Vec<LightningInvoiceResponse> =
            lightning_invoices.into_iter().map(Into::into).collect();

        Ok(response.into())
    }

    async fn delete(
        State(app_state): State<Arc<AppState>>,
        user: AuthUser,
        Path(id): Path<Uuid>,
    ) -> Result<(), ApplicationError> {
        user.check_permission(Permission::WriteLightningTransaction)?;

        app_state.lightning.delete_invoice(id).await?;
        Ok(())
    }

    async fn delete_many(
        State(app_state): State<Arc<AppState>>,
        user: AuthUser,
        Query(query_params): Query<LightningInvoiceFilter>,
    ) -> Result<Json<u64>, ApplicationError> {
        user.check_permission(Permission::WriteLightningTransaction)?;

        let n_deleted = app_state.lightning.delete_invoices(query_params).await?;
        Ok(n_deleted.into())
    }
}
