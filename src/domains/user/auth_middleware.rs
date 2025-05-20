use std::sync::Arc;

use async_trait::async_trait;
use axum::{extract::FromRequestParts, http::request::Parts, RequestPartsExt};
use axum_extra::{
    headers::{authorization::Bearer, Authorization},
    TypedHeader,
};
use base64::{prelude::BASE64_STANDARD, Engine};

use crate::{
    application::errors::{ApplicationError, AuthenticationError},
    infra::app::AppState,
};

use super::User;

#[async_trait]
impl FromRequestParts<Arc<AppState>> for User {
    type Rejection = ApplicationError;

    async fn from_request_parts(parts: &mut Parts, state: &Arc<AppState>) -> Result<Self, Self::Rejection> {
        // Try to extract the Authorization header as Bearer token
        if let Ok(TypedHeader(Authorization(bearer))) = parts.extract::<TypedHeader<Authorization<Bearer>>>().await {
            let user = state.services.auth.authenticate_jwt(bearer.token()).await?;
            Ok(user)
        }
        // Try to extract the Api-Key header
        else if let Some(value) = parts.headers.get("api-key") {
            let value_str = value.to_str().map_err(|_| AuthenticationError::InvalidCredentials)?;
            let api_key = BASE64_STANDARD
                .decode(value_str)
                .map_err(|_| AuthenticationError::InvalidCredentials)?;

            let user = state.services.auth.authenticate_api_key(api_key).await?;
            Ok(user)
        }
        // If no Authorization header is present, return an error
        else {
            Err(AuthenticationError::MissingAuthorizationHeader.into())
        }
    }
}
