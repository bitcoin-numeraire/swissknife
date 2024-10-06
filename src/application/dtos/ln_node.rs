use serde::{Deserialize, Serialize};
use utoipa::ToSchema;

#[derive(Debug, Deserialize, ToSchema)]
pub struct RedeemOnchainRequest {
    /// Recipient BTC address
    pub to_address: String,

    /// Fee rate in sats/vb
    #[schema(example = "8")]
    pub feerate: u32,
}

#[derive(Debug, Serialize, ToSchema)]
pub struct RedeemOnchainResponse {
    /// Transaction ID
    #[schema(example = "ceb662f7e470e6...")]
    pub txid: String,
}

#[derive(Debug, Deserialize, ToSchema)]
pub struct ConnectLSPRequest {
    /// LSP ID
    #[schema(example = "3e8822d5-00de-4fa3-a30e-c2d31f5454e8")]
    pub lsp_id: String,
}

#[derive(Debug, Deserialize, ToSchema)]
pub struct SignMessageRequest {
    /// Message
    #[schema(example = "my message...")]
    pub message: String,
}

#[derive(Debug, Serialize, ToSchema)]
pub struct SignMessageResponse {
    /// zbase encoded signature
    #[schema(example = "d7norubk1xweo96ompcgqg4g4gyy...")]
    pub signature: String,
}

#[derive(Debug, Deserialize, ToSchema)]
pub struct CheckMessageRequest {
    /// Original message
    #[schema(example = "my message...")]
    pub message: String,

    /// zbase encoded signature
    #[schema(example = "d7norubk1xweo96ompcgqg4g4gyy...")]
    pub signature: String,

    /// Node public key
    #[schema(example = "021e15c10d72f86a79323d1e3a42...")]
    pub pubkey: String,
}

#[derive(Debug, Serialize, ToSchema)]
pub struct CheckMessageResponse {
    /// Signature validity
    pub is_valid: bool,
}
