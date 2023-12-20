use crate::adapters::auth::jwt::JWTValidator;

#[derive(Clone, Debug)]
pub struct AppState {
    pub auth_enabled: bool,
    pub jwt_validator: JWTValidator,
}
