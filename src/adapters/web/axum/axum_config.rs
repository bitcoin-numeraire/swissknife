use serde::Deserialize;

#[derive(Clone, Debug, Deserialize)]
pub struct AxumServerConfig {
    pub addr: String,
}
