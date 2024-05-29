use serde::Deserialize;

#[derive(Deserialize)]
pub struct LNUrlpInvoiceQueryParams {
    pub amount: u64,
    pub comment: Option<String>,
}
