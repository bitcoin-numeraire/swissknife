use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use serde_with::{serde_as, DisplayFromStr};
use utoipa::{IntoParams, ToSchema};
use uuid::Uuid;

use crate::{BtcAddress, Invoice, LnAddress, OrderDirection, Payment};

/// A wallet's balance, in millisatoshis.
#[derive(Debug, Clone, Deserialize, Serialize, Default, ToSchema)]
pub struct Balance {
    /// Total amount received
    #[schema(example = 1000000000)]
    pub received_msat: u64,

    /// Total amount sent (settled outgoing payments)
    #[schema(example = 10000000)]
    pub sent_msat: u64,

    /// Total fees paid
    #[schema(example = 1000)]
    pub fees_paid_msat: u64,

    /// Amount reserved for pending outgoing payments
    #[schema(example = 2000)]
    pub reserved_msat: u64,

    /// Amount available to spend.
    #[schema(example = 989999000)]
    pub available_msat: i64,
}

/// A counterparty the wallet has paid, with the date of first contact.
#[derive(Debug, Clone, Deserialize, Serialize, Default, ToSchema)]
pub struct Contact {
    /// Lightning Address
    #[schema(example = "dario_nakamoto@numeraire.tech")]
    pub ln_address: String,

    /// Date of first payment to this contact
    pub contact_since: DateTime<Utc>,
}

/// A spendable asset on one protocol/network.
#[derive(Debug, Clone, Deserialize, Serialize, PartialEq, Eq, ToSchema)]
pub struct Asset {
    /// Internal asset ID
    pub id: Uuid,
    /// Stable server asset code, independent of network-specific UI ticker.
    #[schema(example = "BTC")]
    pub code: String,
    /// Optional display name
    #[serde(skip_serializing_if = "Option::is_none")]
    #[schema(example = "Bitcoin")]
    pub name: Option<String>,
    /// Settlement protocol, such as bitcoin or taproot_assets
    #[schema(example = "bitcoin")]
    pub protocol: String,
    /// Settlement network, such as bitcoin/mainnet or bitcoin/regtest
    #[schema(example = "bitcoin/mainnet")]
    pub network: String,
    /// Protocol-specific asset reference; native for chain-native BTC
    #[schema(example = "native")]
    pub asset_ref: String,
    /// UI ticker for the specific network, such as BTC, tBTC, or rBTC.
    #[schema(example = "BTC")]
    pub display_ticker: String,
    /// Integer storage scale for this asset
    #[schema(example = 11)]
    pub decimals: i16,
    /// Date of creation in database
    pub created_at: DateTime<Utc>,
    #[serde(skip_serializing_if = "Option::is_none")]
    /// Date of update in database
    pub updated_at: Option<DateTime<Utc>>,
}

/// A user wallet with its balance and linked payments, invoices, Bitcoin addresses and contacts.
#[derive(Debug, Clone, Default, Deserialize, Serialize, ToSchema)]
pub struct Wallet {
    /// Internal ID
    pub id: Uuid,
    /// Owning account ID
    pub account_id: Uuid,
    /// Spendable asset held by this wallet
    pub asset_id: Uuid,
    /// Asset metadata for this wallet, when included in the response.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub asset: Option<Asset>,
    /// Optional account-specific display label.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub label: Option<String>,
    /// Lightning Address
    pub ln_address: Option<LnAddress>,
    /// User Balance
    pub balance: Balance,
    /// List of payments
    pub payments: Vec<Payment>,
    /// List of Invoices
    pub invoices: Vec<Invoice>,
    /// List of Bitcoin addresses
    pub btc_addresses: Vec<BtcAddress>,
    /// List of contacts
    pub contacts: Vec<Contact>,
    /// Date of creation in database
    pub created_at: DateTime<Utc>,
    #[serde(skip_serializing_if = "Option::is_none")]
    /// Date of update in database
    pub updated_at: Option<DateTime<Utc>>,
}

/// A lightweight wallet summary with counts in place of the full lists.
#[derive(Debug, Clone, Default, Deserialize, Serialize, ToSchema)]
pub struct WalletOverview {
    /// Internal ID
    pub id: Uuid,
    /// Owning account ID
    pub account_id: Uuid,
    /// Spendable asset held by this wallet
    pub asset_id: Uuid,
    /// Asset metadata for this wallet, when included in the response.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub asset: Option<Asset>,
    /// Optional account-specific display label.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub label: Option<String>,
    /// Lightning Address
    pub ln_address: Option<LnAddress>,
    /// User Balance
    pub balance: Balance,
    /// Number of payments
    pub n_payments: u32,
    /// Number of invoices
    pub n_invoices: u32,
    /// Number of contacts
    pub n_contacts: u32,
    /// Date of creation in database
    pub created_at: DateTime<Utc>,
    #[serde(skip_serializing_if = "Option::is_none")]
    /// Date of update in database
    pub updated_at: Option<DateTime<Utc>>,
}

/// Create Wallet Request
#[derive(Debug, Deserialize, Clone, ToSchema, Serialize)]
pub struct CreateWalletRequest {
    /// Owning account ID
    pub account_id: Uuid,
    /// Asset ID to enable for the account
    pub asset_id: Uuid,
}

/// Wallet query filter.
#[serde_as]
#[derive(Clone, Debug, Deserialize, Serialize, Default, IntoParams)]
pub struct WalletFilter {
    /// Total amount of results to return
    #[serde_as(as = "Option<DisplayFromStr>")]
    pub limit: Option<u64>,
    /// Offset where to start returning results
    #[serde_as(as = "Option<DisplayFromStr>")]
    pub offset: Option<u64>,
    /// List of IDs
    pub ids: Option<Vec<Uuid>>,
    /// Owning account ID
    pub account_id: Option<Uuid>,
    /// Asset ID
    pub asset_id: Option<Uuid>,
    /// Direction of the ordering of results
    #[serde(default)]
    pub order_direction: OrderDirection,
}
