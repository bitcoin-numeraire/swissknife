use std::sync::Arc;

use axum::{
    extract::{Path, Query, State},
    routing::{delete, get, post},
    Json, Router,
};
use uuid::Uuid;

use crate::{
    application::{dtos::NewInvoiceRequest, errors::ApplicationError},
    domains::{
        invoices::entities::{Invoice, InvoiceFilter},
        users::entities::{AuthUser, Permission},
    },
    infra::app::AppState,
};

pub struct InvoiceHandler;

impl InvoiceHandler {
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
    ) -> Result<Json<Invoice>, ApplicationError> {
        user.check_permission(Permission::WriteLightningTransaction)?;

        let ln_address = app_state
            .services
            .invoice
            .invoice(
                payload.user_id.unwrap_or(user.sub),
                payload.amount_msat,
                payload.description,
                payload.expiry,
            )
            .await?;
        Ok(Json(ln_address.into()))
    }

    async fn get(
        State(app_state): State<Arc<AppState>>,
        user: AuthUser,
        Path(id): Path<Uuid>,
    ) -> Result<Json<Invoice>, ApplicationError> {
        user.check_permission(Permission::ReadLightningTransaction)?;

        let ln_address = app_state.services.invoice.get(id).await?;
        Ok(Json(ln_address.into()))
    }

    async fn list(
        State(app_state): State<Arc<AppState>>,
        user: AuthUser,
        Query(query_params): Query<InvoiceFilter>,
    ) -> Result<Json<Vec<Invoice>>, ApplicationError> {
        user.check_permission(Permission::ReadLightningTransaction)?;

        let lightning_invoices = app_state.services.invoice.list(query_params).await?;

        let response: Vec<Invoice> = lightning_invoices.into_iter().map(Into::into).collect();

        Ok(response.into())
    }

    async fn delete(
        State(app_state): State<Arc<AppState>>,
        user: AuthUser,
        Path(id): Path<Uuid>,
    ) -> Result<(), ApplicationError> {
        user.check_permission(Permission::WriteLightningTransaction)?;

        app_state.services.invoice.delete(id).await?;
        Ok(())
    }

    async fn delete_many(
        State(app_state): State<Arc<AppState>>,
        user: AuthUser,
        Query(query_params): Query<InvoiceFilter>,
    ) -> Result<Json<u64>, ApplicationError> {
        user.check_permission(Permission::WriteLightningTransaction)?;

        let n_deleted = app_state.services.invoice.delete_many(query_params).await?;
        Ok(n_deleted.into())
    }
}
