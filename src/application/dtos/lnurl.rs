use serde::Deserialize;
use serde::Serialize;

use crate::domains::invoices::entities::Invoice;

#[derive(Deserialize)]
pub struct LNUrlpInvoiceQueryParams {
    pub amount: u64,             // Amount in milli-satoshis
    pub comment: Option<String>, // Optional comment
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct LNUrlpInvoiceResponse {
    pub pr: String,                            // bech32-serialized lightning invoice
    pub success_action: Option<SuccessAction>, // An optional action to be executed after successfully paying an invoice
    pub disposable: Option<bool>, // An optional flag to let a wallet know whether to persist the link from step 1, if null should be interpreted as true
    pub routes: Vec<String>, // array with payment routes, should be left empty if no routes are to be provided
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SuccessAction {
    pub tag: String,             // action type (url, message, aes, ...)
    pub message: Option<String>, // rest of fields depends on tag value
}

impl From<Invoice> for LNUrlpInvoiceResponse {
    fn from(invoice: Invoice) -> Self {
        Self {
            pr: invoice.lightning.unwrap().bolt11,
            success_action: Some(SuccessAction {
                tag: "message".to_string(),
                message: Some("Thanks for the sats!".to_string()),
            }),
            disposable: None,
            routes: vec![],
        }
    }
}
