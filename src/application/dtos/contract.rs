use rgb_lib::wallet::Recipient;
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

#[derive(Debug, Deserialize)]
pub struct SendAssetsRequest {
    pub recipients: Vec<Recipient>,
    pub fee_rate: f32,
    pub min_confirmations: u8,
}

#[derive(Debug, Deserialize)]
pub struct InvoiceAssetRequest {
    pub asset_id: Option<String>,
    pub amount: Option<u64>,
    pub duration_seconds: Option<u32>,
    pub transport_endpoints: Vec<String>,
    pub min_confirmations: u8,
}
