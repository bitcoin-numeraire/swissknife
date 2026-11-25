use serde::{Deserialize, Serialize};
use serde_with::{serde_as, DisplayFromStr};
use utoipa::ToSchema;

/// An operator-approved resource that this wallet may sell.
#[serde_as]
#[derive(Clone, Serialize, Deserialize, ToSchema)]
pub struct MerchantOrigin {
    pub name: String,
    #[serde_as(as = "DisplayFromStr")]
    #[schema(value_type = String)]
    pub maximum_price_msat: u64,
    /// The wallet price caps per-invoice quotes when enabled.
    pub uses_quote: bool,
    pub access_seconds: u32,
    pub requests_per_payment: Option<u32>,
}

#[serde_as]
#[derive(Clone, Serialize, Deserialize, ToSchema)]
#[serde(deny_unknown_fields)]
pub struct MerchantSaleTerms {
    pub origin: String,
    /// Fixed invoice price, or the ceiling for an origin with a quote callback.
    #[serde_as(as = "DisplayFromStr")]
    #[schema(value_type = String)]
    pub price_msat: u64,
    /// Pausing new invoices preserves previously issued payment terms.
    pub enabled: bool,
}

#[derive(Serialize, Deserialize, ToSchema)]
pub struct WalletMerchantRoute {
    pub name: String,
    pub terms: MerchantSaleTerms,
}

#[derive(Serialize, Deserialize, ToSchema)]
pub struct WalletMerchantCatalog {
    pub origins: Vec<MerchantOrigin>,
    pub routes: Vec<WalletMerchantRoute>,
}
