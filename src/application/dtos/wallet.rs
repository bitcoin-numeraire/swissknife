use serde::Deserialize;

#[derive(Debug, Deserialize)]
pub struct SendBTCRequest {
    pub address: String,
    pub amount: u64,
    pub fee_rate: f32,
}

#[derive(Debug, Deserialize)]
pub struct DrainRequest {
    pub address: String,
    pub fee_rate: f32,
}
