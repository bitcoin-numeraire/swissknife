use nostr_sdk::PublicKey;
use serde::{Deserialize, Serialize};
use utoipa::ToSchema;

#[derive(Debug, Serialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct LnURLPayRequest {
    /// The URL from LN SERVICE to accept the pay request
    #[schema(example = "https://numeraire.tech/lnurlp/dario_nakamoto/callback")]
    pub callback: String,

    /// Max amount in milli-satoshis LN SERVICE is willing to receive
    #[schema(example = 1000000000)]
    pub max_sendable: u64,

    /// Min amount in milli-satoshis LN SERVICE is willing to receive, can not be less than 1 or more than `maxSendable`
    #[schema(example = 1000)]
    pub min_sendable: u64,

    /// Metadata json which must be presented as raw string here, this is required to pass signature verification at a later step
    #[schema(
        example = "[[\"text/plain\",\"dario_nakamoto never refuses sats\"],[\"text/identifier\",\"dario_nakamoto@numeraire.tech\"]]"
    )]
    pub metadata: String,

    /// Optional number of characters accepted for the `comment` query parameter on subsequent callback, defaults to 0 if not provided. (no comment allowed). See <https://github.com/lnurl/luds/blob/luds/12.md>
    #[schema(example = 255)]
    pub comment_allowed: u16,

    /// Type of LNURL
    #[schema(example = "payRequest")]
    pub tag: String,

    /// Nostr enabled
    pub allows_nostr: bool,

    /// Nostr public key
    #[schema(value_type = Option<String>, example = "d9c2ec59a98c...")]
    pub nostr_pubkey: Option<PublicKey>,
}

#[derive(Clone, Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct LnUrlPayRequestData {
    pub callback: String,
    pub min_sendable: u64,
    pub max_sendable: u64,
    pub metadata: String,
    pub tag: String,
    #[serde(default)]
    pub comment_allowed: u16,
    #[serde(default)]
    pub ln_address: Option<String>,
}

impl LnUrlPayRequestData {
    pub fn min_sendable_sats(&self) -> u64 {
        self.min_sendable / 1000
    }

    pub fn max_sendable_sats(&self) -> u64 {
        self.max_sendable / 1000
    }
}

#[derive(Clone, Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct LnUrlPayCallbackResponse {
    pub pr: String,
    pub success_action: Option<LnUrlPaySuccessAction>,
    #[serde(default)]
    pub disposable: Option<bool>,
    #[serde(default)]
    pub routes: Option<Vec<String>>,
}

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

#[derive(Debug)]
pub struct LnUrlCallback {
    pub pr: String,
    pub success_action: Option<LnUrlSuccessAction>,
    pub disposable: Option<bool>,
    pub routes: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, ToSchema, Default)]
pub struct LnUrlSuccessAction {
    /// Action type. One of `url` or `message`
    pub tag: String,

    /// Message displayed to the user
    #[serde(skip_serializing_if = "Option::is_none")]
    pub message: Option<String>,

    /// Description
    #[serde(skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,

    /// URL for the user to open on success
    #[serde(skip_serializing_if = "Option::is_none")]
    pub url: Option<String>,
}
