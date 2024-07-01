use std::sync::Arc;

use axum::{extract::State, routing::post, Json, Router};

use crate::{
    application::{dtos::LoginRequest, errors::ApplicationError},
    infra::app::AppState,
};

pub struct UserHandler;

impl UserHandler {
    pub fn routes() -> Router<Arc<AppState>> {
        Router::new().route("/sign-in", post(Self::sign_in))
    }

    async fn sign_in(
        State(app_state): State<Arc<AppState>>,
        Json(payload): Json<LoginRequest>,
    ) -> Result<String, ApplicationError> {
        let token = app_state.services.user.sign_in(payload.password)?;
        Ok(token)
    }
}
