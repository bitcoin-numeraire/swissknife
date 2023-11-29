#[derive(Debug)]
pub enum ConfigError {
    WebServer(String),
    Wallet(String),
    Lightning(String),
}
