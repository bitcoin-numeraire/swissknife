use serde::Deserialize;

#[derive(Debug, Deserialize)]
pub struct NewInvoiceRequest {
    pub user_id: Option<String>,
    pub amount_msat: u64,
    pub description: Option<String>,
    pub expiry: Option<u32>,
}

#[derive(Debug, Deserialize)]
pub struct SendPaymentRequest {
    pub user_id: Option<String>,
    pub input: String,
    pub amount_msat: Option<u64>,
    pub comment: Option<String>,
}

// Part of the lightning types because this is the payload to send from the node with a swap service
#[derive(Debug, Deserialize)]
pub struct SendOnchainPaymentRequest {
    pub amount_msat: u64,
    pub recipient_address: String,
    pub feerate: u32,
}

#[derive(Debug, Deserialize)]
pub struct RedeemOnchainRequest {
    pub to_address: String,
    pub feerate: u32,
}

#[derive(Debug, Deserialize)]
pub struct RegisterLightningAddressRequest {
    pub user_id: Option<String>,
    pub username: String,
}

#[derive(Debug, Deserialize)]
pub struct ProcessEventRequest {
    pub template: String,
    pub data: EventDataRequest,
}

#[derive(Debug, Deserialize)]
pub struct EventDataRequest {
    pub payment_hash: String,
}
