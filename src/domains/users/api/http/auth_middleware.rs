use async_trait::async_trait;
use axum::{extract::FromRequestParts, http::request::Parts, RequestPartsExt};
use axum_extra::{
    headers::{authorization::Bearer, Authorization},
    TypedHeader,
};
use tracing::debug;

use crate::{
    adapters::{app::AppState, auth::Authenticator},
    application::errors::{ApplicationError, AuthenticationError},
    domains::users::entities::AuthUser,
};

#[async_trait]
impl FromRequestParts<AppState> for AuthUser {
    type Rejection = ApplicationError;

    async fn from_request_parts(
        parts: &mut Parts,
        state: &AppState,
    ) -> Result<Self, Self::Rejection> {
        let jwt_validator = &state.jwt_validator;

        // Check if auth is enabled
        if !state.auth_enabled {
            return Ok(AuthUser::default());
        }

        // Extract the token from the authorization header
        let TypedHeader(Authorization(bearer)) = parts
            .extract::<TypedHeader<Authorization<Bearer>>>()
            .await
            .map_err(|_| {
                ApplicationError::Authentication(AuthenticationError::MissingCredentials(
                    "missing Bearer token for JWT authentication".to_string(),
                ))
            })?;

        // Decode the user data
        let user = jwt_validator.validate(bearer.token()).await?;
        debug!(user = ?user, "Authenticated user");

        Ok(user)
    }
}
