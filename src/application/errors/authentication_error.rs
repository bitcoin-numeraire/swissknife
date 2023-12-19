#[derive(Debug)]
pub enum AuthenticationError {
    RefreshInterval(String),
    JWKS(String),
    JWT(String),
}
