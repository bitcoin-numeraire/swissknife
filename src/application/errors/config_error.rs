#[derive(Debug)]
pub enum ConfigError {
    Load(String),
    WebServer(String),
    Wallet(String),
    Lightning(String),
}
