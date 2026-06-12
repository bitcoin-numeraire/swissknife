use serde::{Deserialize, Serialize};
use utoipa::ToSchema;

/// LNURL-pay callback response. Carries the invoice to pay and how to behave on
/// success. Wire shape follows LUD-06 (camelCase fields).
#[derive(Debug, Clone, Serialize, ToSchema)]
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
