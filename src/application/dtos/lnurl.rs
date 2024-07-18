use serde::{Deserialize, Serialize};
use serde_json::Value;
use utoipa::{IntoParams, ToSchema};

use crate::domains::lnurl::LnUrlCallback;

#[derive(Debug, Deserialize, IntoParams)]
pub struct LNUrlpInvoiceQueryParams {
    /// Amount in millisatoshis
    pub amount: u64,
    /// Optional comment for the recipient
    pub comment: Option<String>,
}

#[derive(Debug, Serialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct LnUrlCallbackResponse {
    /// bech32-serialized Lightning invoice
    #[schema(example = "lnbcrt1m1png24kasp5...")]
    pub pr: String,
    /// An optional action to be executed after successfully paying an invoice
    pub success_action: Option<Value>,
    /// An optional flag to let a wallet know whether to persist the link from step 1, if null should be interpreted as true
    pub disposable: Option<bool>,
    /// array with payment routes, should be left empty if no routes are to be provided
    pub routes: Vec<String>,
}

impl From<LnUrlCallback> for LnUrlCallbackResponse {
    fn from(callback: LnUrlCallback) -> Self {
        LnUrlCallbackResponse {
            pr: callback.pr,
            success_action: callback
                .success_action
                .map(|sa| serde_json::to_value(sa).unwrap_or(Value::Null)),
            disposable: callback.disposable,
            routes: callback.routes,
        }
    }
}
