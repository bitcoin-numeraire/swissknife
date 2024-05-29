use serde::Serialize;

/// See <https://github.com/lnurl/luds/blob/luds/06.md>
#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct LnURLPayRequest {
    pub callback: String,
    pub max_sendable: u64, // Max amount in milli-satoshis LN SERVICE is willing to receive
    pub min_sendable: u64, // Min amount in milli-satoshis LN SERVICE is willing to receive, can not be less than 1 or more than `maxSendable`
    pub metadata: String, // Metadata json which must be presented as raw string here, this is required to pass signature verification at a later step
    pub comment_allowed: u16, // Optional number of characters accepted for the `comment` query parameter on subsequent callback, defaults to 0 if not provided. (no comment allowed). See <https://github.com/lnurl/luds/blob/luds/12.md>
    pub tag: String,          // Type of LNURL
}
