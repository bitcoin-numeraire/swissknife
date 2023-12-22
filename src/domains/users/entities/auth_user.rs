#[derive(Clone, Debug)]
pub struct AuthUser {
    pub sub: String,
}

impl Default for AuthUser {
    fn default() -> Self {
        Self {
            sub: "anonymous".to_string(),
        }
    }
}
