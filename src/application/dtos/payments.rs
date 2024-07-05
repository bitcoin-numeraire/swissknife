use serde::Deserialize;
use utoipa::ToSchema;

/// Send Payment Request
#[derive(Debug, Deserialize, ToSchema)]
pub struct SendPaymentRequest {
    /// User ID. Will be populated with your own ID by default
    pub user_id: Option<String>,
    /// Recipient. Can be a Bolt11 invoice, LNURL or LN Address. Keysend and On-chain payments not yet supported
    pub input: String,
    /// Amount in millisatoshis. Only necessary if the input does not specify an amount (empty Bolt11, LNURL or LN Address)
    pub amount_msat: Option<u64>,
    /// Comment of the payment. Visible by the recipient for LNURL payments
    pub comment: Option<String>,
}

/// Send On-chain Payment Request
#[derive(Debug, Deserialize, ToSchema)]
pub struct SendOnchainPaymentRequest {
    /// Amount in millisatoshis
    pub amount_msat: u64,
    /// Recipient Bitcoin address
    pub recipient_address: String,
    /// Fee rate in sats/vb
    pub feerate: u32,
}
