use serde::Deserialize;
use utoipa::ToSchema;

/// New Invoice Request
#[derive(Debug, Deserialize, ToSchema)]
pub struct NewInvoiceRequest {
    /// User ID. Will be populated with your own ID by default
    pub user_id: Option<String>,
    /// Amount in millisatoshis
    pub amount_msat: u64,
    /// Description of the invoice. Visible by the payer
    pub description: Option<String>,
    /// Expiration time in seconds
    pub expiry: Option<u32>,
}
