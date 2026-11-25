use serde::{Deserialize, Serialize};
use strum_macros::{Display, EnumString};
use utoipa::ToSchema;

/// The ledger a transaction settles on.
#[derive(Clone, Debug, EnumString, Deserialize, Serialize, Display, PartialEq, Eq, Default, ToSchema)]
pub enum Ledger {
    #[default]
    Lightning,
    Internal,
    Onchain,
}

/// The currency and network a transaction is denominated in.
#[derive(Clone, Debug, EnumString, Deserialize, Serialize, Display, PartialEq, Eq, Default, ToSchema)]
pub enum Currency {
    #[default]
    Bitcoin,
    BitcoinTestnet,
    Regtest,
    Simnet,
    Signet,
}

/// Wallet presentation fields attached to administrative transaction listings.
#[derive(Clone, Debug, Serialize, Deserialize, ToSchema)]
pub struct TransactionWalletDisplay {
    pub wallet_id: uuid::Uuid,
    pub wallet_label: Option<String>,
    pub username: Option<String>,
    pub display_ticker: String,
    pub network: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub account_id: Option<uuid::Uuid>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub account_label: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub account_subject: Option<String>,
}

/// An administrative invoice retains the original invoice fields on the wire.
#[derive(Clone, Debug, Serialize, Deserialize, ToSchema)]
pub struct AdminInvoiceSummary {
    #[serde(flatten)]
    pub invoice: crate::Invoice,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub wallet_display: Option<TransactionWalletDisplay>,
}

/// An administrative payment retains the original payment fields on the wire.
#[derive(Clone, Debug, Serialize, Deserialize, ToSchema)]
pub struct AdminPaymentSummary {
    #[serde(flatten)]
    pub payment: crate::Payment,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub wallet_display: Option<TransactionWalletDisplay>,
}
