use serde::Serialize;
use utoipa::ToSchema;

use crate::domains::ln_address::LnAddress;

pub use swissknife_types::{RegisterLnAddressRequest, UpdateLnAddressRequest};

#[derive(Debug, Serialize, ToSchema)]
pub struct WalletLnAddressResponse {
    /// Wallet LN address. Empty if not found
    pub ln_address: Option<LnAddress>,
}
