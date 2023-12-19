#[derive(Debug)]
pub enum AuthenticationError {
    JWKS(String),
    JWT(String),
}
