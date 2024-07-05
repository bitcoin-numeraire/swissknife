use serde::Deserialize;
use utoipa::ToSchema;

#[derive(Debug, Deserialize, ToSchema)]
pub struct RedeemOnchainRequest {
    pub to_address: String,
    pub feerate: u32,
}

#[derive(Debug, Deserialize, ToSchema)]
pub struct RegisterLightningAddressRequest {
    pub user_id: Option<String>,
    pub username: String,
}

#[derive(Debug, Deserialize, ToSchema)]
pub struct ConnectLSPRequest {
    pub lsp_id: String,
}

#[derive(Debug, Deserialize, ToSchema)]
pub struct SignMessageRequest {
    pub message: String,
}

#[derive(Debug, Deserialize, ToSchema)]
pub struct CheckMessageRequest {
    pub message: String,
    pub signature: String,
    pub pubkey: String,
}
