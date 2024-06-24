use std::sync::Arc;

use axum::{extract::State, routing::post, Json, Router};

use crate::{
    application::{dtos::LoginRequest, errors::ApplicationError},
    infra::app::AppState,
};

pub struct UserHandler;

impl UserHandler {
    pub fn routes() -> Router<Arc<AppState>> {
        Router::new().route("/login", post(Self::login))
    }

    async fn login(
        State(app_state): State<Arc<AppState>>,
        Json(payload): Json<LoginRequest>,
    ) -> Result<String, ApplicationError> {
        let token = app_state.services.user.login(payload.password)?;
        Ok(token)
    }
}
