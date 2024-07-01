use serde::Deserialize;

#[derive(Debug, Deserialize)]
pub struct LoginRequest {
    pub password: String,
}
