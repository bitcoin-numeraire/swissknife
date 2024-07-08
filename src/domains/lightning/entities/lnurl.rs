use breez_sdk_core::SuccessActionProcessed;
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
}

#[derive(Deserialize, Debug, Serialize)]
pub struct LnUrlErrorData {
    pub reason: String,
}

#[derive(Debug, Serialize, Deserialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct LnUrlCallbackResponse {
    /// bech32-serialized Lightning invoice
    #[schema(example = "lnbcrt1m1png24kasp5...")]
    pub pr: String,
    /// An optional action to be executed after successfully paying an invoice
    pub success_action: Option<SuccessActionProcessed>,
    /// An optional flag to let a wallet know whether to persist the link from step 1, if null should be interpreted as true
    pub disposable: Option<bool>,
    /// array with payment routes, should be left empty if no routes are to be provided
    pub routes: Vec<String>,
}
