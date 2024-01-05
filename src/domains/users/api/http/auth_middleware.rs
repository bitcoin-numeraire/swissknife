use std::sync::Arc;

use async_trait::async_trait;
use axum::{extract::FromRequestParts, http::request::Parts, RequestPartsExt};
use axum_extra::{
    headers::{authorization::Bearer, Authorization},
    TypedHeader,
};
use tracing::{debug, trace};

use crate::{
    adapters::auth::Authenticator, application::errors::AuthenticationError,
    domains::users::entities::AuthUser,
};

#[async_trait]
impl FromRequestParts<Option<Arc<dyn Authenticator>>> for AuthUser {
    type Rejection = AuthenticationError;

    async fn from_request_parts(
        parts: &mut Parts,
        jwt_authenticator: &Option<Arc<dyn Authenticator>>,
    ) -> Result<Self, Self::Rejection> {
        trace!("Start authentication");

        if jwt_authenticator.is_none() {
            return Ok(AuthUser::default());
        }

        let jwt_authenticator = jwt_authenticator.as_ref().unwrap();

        // Extract the token from the Authorization header
        let TypedHeader(Authorization(bearer)) = parts
            .extract::<TypedHeader<Authorization<Bearer>>>()
            .await
            .map_err(|e| {
                let err_message = "Missing Bearer token for JWT authentication";
                debug!(error = ?e, err_message);
                AuthenticationError::MissingCredentials(e.to_string())
            })?;

        // Decode the user data
        let user = jwt_authenticator.authenticate(bearer.token()).await?;
        debug!(user = ?user, "Authentication successful");

        Ok(user)
    }
}
