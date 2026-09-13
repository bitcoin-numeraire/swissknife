use std::sync::Arc;

use axum::{
    extract::FromRequestParts,
    http::{request::Parts, HeaderMap},
};
use axum_extra::headers::{authorization::Bearer, Authorization, HeaderMapExt};
use base64::{prelude::BASE64_STANDARD, Engine};

use crate::application::{
    composition::AppServices,
    errors::{ApplicationError, AuthenticationError},
};

use super::User;

impl FromRequestParts<Arc<AppServices>> for User {
    type Rejection = ApplicationError;

    async fn from_request_parts(parts: &mut Parts, services: &Arc<AppServices>) -> Result<Self, Self::Rejection> {
        Self::authenticate_headers(&parts.headers, services).await
    }
}

impl User {
    pub(crate) async fn authenticate_headers(
        headers: &HeaderMap,
        services: &AppServices,
    ) -> Result<Self, ApplicationError> {
        if let Some(Authorization(bearer)) = headers.typed_get::<Authorization<Bearer>>() {
            let user = services.auth.authenticate_jwt(bearer.token()).await?;
            Ok(user)
        }
        // Try to extract the Api-Key header
        else if let Some(value) = headers.get("api-key") {
            let value_str = value.to_str().map_err(|_| AuthenticationError::InvalidCredentials)?;
            let api_key = BASE64_STANDARD
                .decode(value_str)
                .map_err(|_| AuthenticationError::InvalidCredentials)?;

            let user = services.auth.authenticate_api_key(api_key).await?;
            Ok(user)
        }
        // If no Authorization header is present, return an error
        else {
            Err(AuthenticationError::MissingAuthorizationHeader.into())
        }
    }
}
