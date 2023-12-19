use serde::Deserialize;

use super::jwt::JWTConfig;

#[derive(Clone, Debug, Deserialize)]
pub struct AuthConfig {
    enabled: bool,
    jwt: JWTConfig,
}

pub trait Authenticator {}
