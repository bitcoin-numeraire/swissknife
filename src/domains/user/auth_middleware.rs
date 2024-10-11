use std::sync::Arc;

use async_trait::async_trait;
use axum::{
    extract::FromRequestParts,
    http::{request::Parts, HeaderValue},
    RequestPartsExt,
};
use axum_extra::{
    headers::{
        authorization::{Bearer, Credentials},
        Authorization,
    },
    TypedHeader,
};
use base64::{prelude::BASE64_STANDARD, Engine};

use crate::{
    application::errors::{ApplicationError, AuthenticationError},
    infra::app::AppState,
};

use super::User;

#[derive(Clone, Debug)]
pub struct ApiKey(pub Vec<u8>);

impl Credentials for ApiKey {
    const SCHEME: &'static str = "Api-Key";

    fn decode(value: &HeaderValue) -> Option<Self> {
        let value_str = value.to_str().ok()?;

        // Expected format: "Api-Key base64_encoded_api_key"
        // Check that it starts with "Api-Key " (case-insensitive)
        let scheme_with_space = format!("{} ", Self::SCHEME);
        if !value_str
            .get(..scheme_with_space.len())?
            .eq_ignore_ascii_case(&scheme_with_space)
        {
            return None;
        }

        let base64_part = value_str[scheme_with_space.len()..].trim_start();
        let bytes = BASE64_STANDARD.decode(base64_part).ok()?;

        Some(ApiKey(bytes))
    }

    fn encode(&self) -> HeaderValue {
        let base64_encoded = BASE64_STANDARD.encode(&self.0);
        let value = format!("{} {}", Self::SCHEME, base64_encoded);
        HeaderValue::from_str(&value).expect("Base64 encoding is always a valid HeaderValue")
    }
}

#[async_trait]
impl FromRequestParts<Arc<AppState>> for User {
    type Rejection = ApplicationError;

    async fn from_request_parts(
        parts: &mut Parts,
        state: &Arc<AppState>,
    ) -> Result<Self, Self::Rejection> {
        // Try to extract the Authorization header as Bearer token
        if let Ok(TypedHeader(Authorization(bearer))) =
            parts.extract::<TypedHeader<Authorization<Bearer>>>().await
        {
            let user = state.services.auth.authenticate_jwt(bearer.token()).await?;
            Ok(user)
        }
        // Try to extract the Authorization header as ApiKey
        else if let Ok(TypedHeader(Authorization(api_key))) =
            parts.extract::<TypedHeader<Authorization<ApiKey>>>().await
        {
            let user = state.services.auth.authenticate_api_key(api_key.0).await?;
            Ok(user)
        }
        // If no Authorization header is present, return an error
        else {
            Err(AuthenticationError::MissingAuthorizationHeader.into())
        }
    }
}
