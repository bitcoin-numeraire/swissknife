use serde::{Deserialize, Serialize};

pub struct RGBAsset {
    pub asset_type: RGBAssetType,
    pub ticker: String, // Ticker for NIA and UDA assets
    pub name: String,
    pub details: Option<String>, // Details for UDA assets
    pub precision: u8,
    pub amounts: Vec<u64>,        // Amounts issued for NIA and CFA assets
    pub filename: Option<String>, // Media file name for UDA and CFA assets
}

#[derive(Debug, Deserialize, Serialize, Clone)]
pub enum RGBAssetType {
    NIA,
    CFA,
    UDA,
}

#[derive(Debug, Deserialize, Serialize, Clone)]
pub enum RGBInvoiceType {
    BLIND,
    WITNESS,
}
