use serde::Deserialize;

use crate::domains::rgb::entities::{RGBAssetType, RGBInvoiceType};

#[derive(Debug, Deserialize)]
pub struct PrepareIssuanceRequest {
    pub fee_rate: Option<f32>,
}

#[derive(Debug, Deserialize)]
pub struct IssueAssetRequest {
    pub asset_type: RGBAssetType,
    pub ticker: Option<String>, // Ticker for NIA and UDA assets
    pub name: String,
    pub details: Option<String>, // Details for UDA assets
    pub precision: Option<u8>,
    pub amount: Option<u64>,      // Amount issued for NIA and CFA assets
    pub filename: Option<String>, // Media file name for UDA and CFA assets
    pub recipient: Option<String>,
    pub fee_rate: Option<f32>,
}

#[derive(Debug, Deserialize)]
pub struct InvoiceAssetRequest {
    pub invoice_type: RGBInvoiceType,
    pub asset_id: Option<String>,
    pub amount: Option<u64>,
    pub duration_seconds: Option<u32>,
}

#[derive(Debug, Deserialize)]
pub struct SendAssetsRequest {
    pub recipient: String,
    pub amount: Option<u64>,
    pub fee_rate: Option<f32>,
}
