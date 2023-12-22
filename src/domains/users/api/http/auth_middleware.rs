use std::sync::Arc;

use async_trait::async_trait;
use axum::{extract::FromRequestParts, http::request::Parts, RequestPartsExt};
use axum_extra::{
    headers::{authorization::Bearer, Authorization},
    TypedHeader,
};
use tracing::{debug, trace};

use crate::{
    adapters::app::AppState, application::errors::AuthenticationError,
    domains::users::entities::AuthUser,
};

#[async_trait]
impl FromRequestParts<Arc<AppState>> for AuthUser {
    type Rejection = AuthenticationError;

    async fn from_request_parts(
        parts: &mut Parts,
        state: &Arc<AppState>,
    ) -> Result<Self, Self::Rejection> {
        trace!("Start authentication");

        let jwt_validator = &state.jwt_validator;

        // Check if auth is enabled
        if !state.auth_enabled {
            debug!("Authentication disabled, returning anonymous user");
            return Ok(AuthUser::default());
        }

        // Extract the token from the authorization header
        let TypedHeader(Authorization(bearer)) = parts
            .extract::<TypedHeader<Authorization<Bearer>>>()
            .await
            .map_err(|e| {
                let err_message = "Missing Bearer token for JWT authentication";
                debug!(error = ?e, err_message);
                AuthenticationError::MissingCredentials(e.to_string())
            })?;

        // Decode the user data
        let user = jwt_validator.validate(bearer.token()).await?;
        debug!(user = ?user, "Authentication successful");

        Ok(user)
    }
}
