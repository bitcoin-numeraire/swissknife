use serde::Deserialize;
use utoipa::IntoParams;

#[derive(Debug, Deserialize, IntoParams)]
pub struct LNUrlpInvoiceQueryParams {
    /// Amount in millisatoshis
    pub amount: u64,
    /// Optional comment for the recipient
    pub comment: Option<String>,
}
