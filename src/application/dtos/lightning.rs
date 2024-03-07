use serde::Deserialize;

#[derive(Debug, Deserialize)]
pub struct SendPaymentRequest {
    pub bolt11: String,
    pub amount_msat: Option<u64>,
}
