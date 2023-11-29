#[derive(Debug)]
pub enum ConfigError {
    Wallet(String),
    Lightning(String),
}
