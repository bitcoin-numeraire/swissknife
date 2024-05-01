use std::sync::Arc;

use axum::{
    extract::{Path, Query, State},
    routing::{get, post},
    Json, Router,
};

use crate::{
    application::{
        dtos::{LightningAddressResponse, PaginationQueryParams, RegisterLightningAddressRequest},
        errors::{ApplicationError, DataError},
    },
    domains::users::entities::AuthUser,
    infra::app::AppState,
};

const MIN_USERNAME_LENGTH: usize = 1;
const MAX_USERNAME_LENGTH: usize = 64;

pub struct LightningAddressHandler;

impl LightningAddressHandler {
    pub fn routes() -> Router<Arc<AppState>> {
        Router::new()
            .route("/", get(Self::list))
            .route("/", post(Self::register))
            .route("/:username", get(Self::get))
    }

    async fn register(
        State(app_state): State<Arc<AppState>>,
        user: AuthUser,
        Json(payload): Json<RegisterLightningAddressRequest>,
    ) -> Result<Json<LightningAddressResponse>, ApplicationError> {
        let username_length = payload.username.len();
        if username_length < MIN_USERNAME_LENGTH || username_length > MAX_USERNAME_LENGTH {
            return Err(
                DataError::RequestValidation("Invalid username length.".to_string()).into(),
            );
        }

        let lightning_address = app_state
            .lightning
            .register_address(user, payload.username)
            .await?;

        Ok(Json(lightning_address.into()))
    }

    async fn get(
        State(app_state): State<Arc<AppState>>,
        user: AuthUser,
        Path(username): Path<String>,
    ) -> Result<Json<LightningAddressResponse>, ApplicationError> {
        let lightning_address = app_state.lightning.get_address(user, username).await?;

        Ok(Json(lightning_address.into()))
    }

    async fn list(
        State(app_state): State<Arc<AppState>>,
        user: AuthUser,
        Query(query_params): Query<PaginationQueryParams>,
    ) -> Result<Json<Vec<LightningAddressResponse>>, ApplicationError> {
        let lightning_addresses = app_state
            .lightning
            .list_addresses(user, query_params.limit, query_params.offset)
            .await?;

        let response: Vec<LightningAddressResponse> =
            lightning_addresses.into_iter().map(Into::into).collect();

        Ok(response.into())
    }
}
