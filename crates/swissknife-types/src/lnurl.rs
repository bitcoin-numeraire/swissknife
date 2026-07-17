use nostr::PublicKey;
use serde::{Deserialize, Serialize};
use utoipa::{IntoParams, ToSchema};

/// LNURL-pay callback response. Carries the invoice to pay and how to behave on
/// success. Wire shape follows LUD-06 (camelCase fields).
#[derive(Debug, Clone, Deserialize, Serialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct LnUrlCallback {
    /// bech32-serialized Lightning invoice
    #[schema(example = "lnbcrt1m1png24kasp5...")]
    pub pr: String,
    /// An optional action to be executed after successfully paying an invoice
    pub success_action: Option<LnUrlSuccessAction>,
    /// An optional flag to let a wallet know whether to persist the link from step 1, if null should be interpreted as true
    pub disposable: Option<bool>,
    /// array with payment routes, should be left empty if no routes are to be provided
    pub routes: Vec<String>,
}

/// LNURL success action shown to the payer after a successful payment (LUD-09).
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

/// Validated LNURL success action retained internally until payment settlement.
///
/// This is deliberately not part of the public payment JSON contract. AES
/// actions must remain encrypted until the Lightning preimage is available.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
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

/// LNURL-pay `payRequest` response served at the well-known endpoint (LUD-06).
#[derive(Debug, Deserialize, Serialize, ToSchema)]
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

/// LNURL-pay callback query parameters
#[derive(Debug, Deserialize, IntoParams)]
pub struct LNUrlpInvoiceQueryParams {
    /// Amount in millisatoshis
    pub amount: u64,
    /// Optional comment for the recipient
    pub comment: Option<String>,
}
