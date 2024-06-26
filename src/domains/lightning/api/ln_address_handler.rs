use std::sync::Arc;

use axum::{
    extract::{Path, State},
    routing::{delete, get, post},
    Json, Router,
};
use axum_extra::extract::Query;
use uuid::Uuid;

use crate::{
    application::{dtos::RegisterLightningAddressRequest, errors::ApplicationError},
    domains::{
        lightning::entities::{LnAddress, LnAddressFilter},
        users::entities::{AuthUser, Permission},
    },
    infra::app::AppState,
};

pub struct LnAddressHandler;

impl LnAddressHandler {
    pub fn routes() -> Router<Arc<AppState>> {
        Router::new()
            .route("/", get(Self::list))
            .route("/", post(Self::register))
            .route("/:id", get(Self::get))
            .route("/:id", delete(Self::delete))
            .route("/", delete(Self::delete_many))
    }

    async fn register(
        State(app_state): State<Arc<AppState>>,
        user: AuthUser,
        Json(payload): Json<RegisterLightningAddressRequest>,
    ) -> Result<Json<LnAddress>, ApplicationError> {
        user.check_permission(Permission::WriteLnAddress)?;

        let ln_address = app_state
            .services
            .lnurl
            .register(payload.user_id.unwrap_or(user.sub), payload.username)
            .await?;
        Ok(Json(ln_address.into()))
    }

    async fn get(
        State(app_state): State<Arc<AppState>>,
        user: AuthUser,
        Path(id): Path<Uuid>,
    ) -> Result<Json<LnAddress>, ApplicationError> {
        user.check_permission(Permission::ReadLnAddress)?;

        let ln_address = app_state.services.lnurl.get(id).await?;
        Ok(Json(ln_address.into()))
    }

    async fn list(
        State(app_state): State<Arc<AppState>>,
        user: AuthUser,
        Query(query_params): Query<LnAddressFilter>,
    ) -> Result<Json<Vec<LnAddress>>, ApplicationError> {
        user.check_permission(Permission::ReadLnAddress)?;

        let ln_addresses = app_state.services.lnurl.list(query_params).await?;

        let response: Vec<LnAddress> = ln_addresses.into_iter().map(Into::into).collect();

        Ok(response.into())
    }

    async fn delete(
        State(app_state): State<Arc<AppState>>,
        user: AuthUser,
        Path(id): Path<Uuid>,
    ) -> Result<(), ApplicationError> {
        user.check_permission(Permission::WriteLnAddress)?;

        app_state.services.lnurl.delete(id).await?;
        Ok(())
    }

    async fn delete_many(
        State(app_state): State<Arc<AppState>>,
        user: AuthUser,
        Query(query_params): Query<LnAddressFilter>,
    ) -> Result<Json<u64>, ApplicationError> {
        user.check_permission(Permission::WriteLnAddress)?;

        let n_deleted = app_state.services.lnurl.delete_many(query_params).await?;
        Ok(n_deleted.into())
    }
}
