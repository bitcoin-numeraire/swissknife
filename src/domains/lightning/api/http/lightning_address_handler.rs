use std::sync::Arc;

use axum::{
    extract::{Path, Query, State},
    routing::{get, post},
    Json, Router,
};
use uuid::Uuid;

use crate::{
    application::{dtos::RegisterLightningAddressRequest, errors::ApplicationError},
    domains::{
        lightning::entities::{LightningAddress, LightningAddressFilter},
        users::entities::{AuthUser, Permission},
    },
    infra::app::AppState,
};

pub struct LightningAddressHandler;

impl LightningAddressHandler {
    pub fn routes() -> Router<Arc<AppState>> {
        Router::new()
            .route("/", get(Self::list))
            .route("/", post(Self::register))
            .route("/:id", get(Self::get))
    }

    async fn register(
        State(app_state): State<Arc<AppState>>,
        user: AuthUser,
        Json(payload): Json<RegisterLightningAddressRequest>,
    ) -> Result<Json<LightningAddress>, ApplicationError> {
        user.check_permission(Permission::WriteLightningAddress)?;

        let lightning_address = app_state
            .lightning
            .register_address(payload.user_id.unwrap_or(user.sub), payload.username)
            .await?;
        Ok(Json(lightning_address.into()))
    }

    async fn get(
        State(app_state): State<Arc<AppState>>,
        user: AuthUser,
        Path(id): Path<Uuid>,
    ) -> Result<Json<LightningAddress>, ApplicationError> {
        user.check_permission(Permission::ReadLightningAddress)?;

        let lightning_address = app_state.lightning.get_address(id).await?;
        Ok(Json(lightning_address.into()))
    }

    async fn list(
        State(app_state): State<Arc<AppState>>,
        user: AuthUser,
        Query(query_params): Query<LightningAddressFilter>,
    ) -> Result<Json<Vec<LightningAddress>>, ApplicationError> {
        user.check_permission(Permission::ReadLightningAddress)?;

        let lightning_addresses = app_state.lightning.list_addresses(query_params).await?;

        let response: Vec<LightningAddress> =
            lightning_addresses.into_iter().map(Into::into).collect();

        Ok(response.into())
    }
}
