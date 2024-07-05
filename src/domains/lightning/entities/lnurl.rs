use breez_sdk_core::SuccessActionProcessed;
use serde::{Deserialize, Serialize};
use utoipa::ToSchema;

/// See <https://github.com/lnurl/luds/blob/luds/06.md>
#[derive(Debug, Serialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct LnURLPayRequest {
    pub callback: String,
    pub max_sendable: u64, // Max amount in milli-satoshis LN SERVICE is willing to receive
    pub min_sendable: u64, // Min amount in milli-satoshis LN SERVICE is willing to receive, can not be less than 1 or more than `maxSendable`
    pub metadata: String, // Metadata json which must be presented as raw string here, this is required to pass signature verification at a later step
    pub comment_allowed: u16, // Optional number of characters accepted for the `comment` query parameter on subsequent callback, defaults to 0 if not provided. (no comment allowed). See <https://github.com/lnurl/luds/blob/luds/12.md>
    pub tag: String,          // Type of LNURL
}

#[derive(Deserialize, Debug, Serialize)]
pub struct LnUrlErrorData {
    pub reason: String,
}

#[derive(Debug, Serialize, Deserialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct LnUrlCallbackResponse {
    pub pr: String, // bech32-serialized lightning invoice
    pub success_action: Option<SuccessActionProcessed>, // An optional action to be executed after successfully paying an invoice
    pub disposable: Option<bool>, // An optional flag to let a wallet know whether to persist the link from step 1, if null should be interpreted as true
    pub routes: Vec<String>, // array with payment routes, should be left empty if no routes are to be provided
}
