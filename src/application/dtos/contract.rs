use serde::Deserialize;
use serde::Serialize;

#[derive(Debug, Deserialize)]
pub struct PrepareIssuanceRequest {
    pub fee_rate: f32,
}

#[derive(Debug, Deserialize)]
pub struct IssueContractRequest {
    pub ticker: String,
    pub name: String,
    pub precision: u8,
    pub amounts: Vec<u64>,
}

#[derive(Debug, Serialize)]
pub struct ContractResponse {
    pub id: String,
}
