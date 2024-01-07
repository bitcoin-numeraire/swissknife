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

        if state.jwt_authenticator.is_none() {
            return Ok(AuthUser::default());
        }

        let jwt_authenticator = state.jwt_authenticator.as_ref().unwrap();

        // Extract the token from the Authorization header
        let TypedHeader(Authorization(bearer)) = parts
            .extract::<TypedHeader<Authorization<Bearer>>>()
            .await
            .map_err(|e| AuthenticationError::MissingBearerToken(e.to_string()))?;

        // Decode the user data
        trace!(token = bearer.token(), "Start JWT validation");

        let user = jwt_authenticator.authenticate(bearer.token()).await?;

        debug!(user = ?user, "Authentication successful");
        Ok(user)
    }
}
