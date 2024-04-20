use serde::Deserialize;

#[derive(Clone, Debug, Deserialize)]
pub struct AxumServerConfig {
    pub addr: String,
    pub request_timeout: String,
}
