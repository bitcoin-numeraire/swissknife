use std::sync::Arc;

use async_trait::async_trait;
use axum::{extract::FromRequestParts, http::request::Parts, RequestPartsExt};
use axum_extra::{
    headers::{authorization::Bearer, Authorization},
    TypedHeader,
};

use crate::{
    application::{
        dtos::AuthProvider,
        errors::{ApplicationError, AuthenticationError},
    },
    infra::app::AppState,
};

use super::Account;

#[async_trait]
impl FromRequestParts<Arc<AppState>> for Account {
    type Rejection = ApplicationError;

    async fn from_request_parts(
        parts: &mut Parts,
        state: &Arc<AppState>,
    ) -> Result<Self, Self::Rejection> {
        let credentials = match state.services.auth.provider() {
            AuthProvider::Bypass => "".to_string(),
            _ => {
                // Extract the token from the Authorization header
                let TypedHeader(Authorization(bearer)) = parts
                    .extract::<TypedHeader<Authorization<Bearer>>>()
                    .await
                    .map_err(|e| AuthenticationError::MissingBearerToken(e.to_string()))?;

                bearer.token().to_string()
            }
        };

        let user = state.services.auth.authenticate(&credentials).await?;

        Ok(user)
    }
}
