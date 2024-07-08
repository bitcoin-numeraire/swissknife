use serde::Deserialize;
use utoipa::ToSchema;

/// Send Payment Request
#[derive(Debug, Deserialize, ToSchema)]
pub struct SendPaymentRequest {
    /// User ID. Will be populated with your own ID by default
    pub user_id: Option<String>,

    /// Recipient. Can be a Bolt11 invoice, LNURL or LN Address. Keysend and On-chain payments not yet supported
    #[schema(example = "hello@numeraire.tech")]
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
    #[schema(example = 100000000)]
    pub amount_msat: u64,

    /// Recipient Bitcoin address
    #[schema(example = "bc1q7jys2n3jjf9t25r6ut369taap8v38pgqekq8v4")]
    pub recipient_address: String,

    /// Fee rate in sats/vb
    #[schema(example = "8")]
    pub feerate: u32,
}
