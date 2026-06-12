use serde::Deserialize;

pub use swissknife_types::{LnURLPayRequest, LnUrlCallback, LnUrlSuccessAction};

/// Parsed `payRequest` from a remote LNURL service, used when paying out.
#[derive(Clone, Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct LnUrlPayRequestData {
    pub callback: String,
    pub min_sendable: u64,
    pub max_sendable: u64,
    pub metadata: String,
    #[serde(default)]
    pub comment_allowed: u16,
    #[serde(default)]
    pub ln_address: Option<String>,
}

/// Parsed callback response from a remote LNURL service.
#[derive(Clone, Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct LnUrlPayCallbackResponse {
    pub pr: String,
    pub success_action: Option<LnUrlPaySuccessAction>,
}

/// Success action parsed from a remote LNURL service.
#[derive(Clone, Debug, Deserialize)]
#[serde(tag = "tag", rename_all = "camelCase")]
pub enum LnUrlPaySuccessAction {
    #[serde(rename = "message")]
    Message { message: String },
    #[serde(rename = "url")]
    Url { description: String, url: String },
    #[serde(rename = "aes")]
    Aes {
        description: String,
        ciphertext: String,
        iv: String,
    },
}
