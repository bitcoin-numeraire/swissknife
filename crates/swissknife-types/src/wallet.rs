use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use serde_with::{serde_as, DisplayFromStr};
use utoipa::{IntoParams, ToSchema};
use uuid::Uuid;

use crate::{BtcAddress, BtcNetwork, Invoice, LnAddress, OrderDirection, Payment};

/// Asset settlement protocol.
#[derive(Debug, Clone, Copy, Default, Deserialize, Serialize, PartialEq, Eq, ToSchema)]
pub enum AssetProtocol {
    #[default]
    #[serde(rename = "bitcoin")]
    Bitcoin,
    #[serde(rename = "taproot_assets")]
    TaprootAssets,
}

impl AssetProtocol {
    pub const fn as_str(self) -> &'static str {
        match self {
            AssetProtocol::Bitcoin => "bitcoin",
            AssetProtocol::TaprootAssets => "taproot_assets",
        }
    }
}

impl std::fmt::Display for AssetProtocol {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(self.as_str())
    }
}

impl std::str::FromStr for AssetProtocol {
    type Err = String;

    fn from_str(value: &str) -> Result<Self, Self::Err> {
        match value {
            "bitcoin" => Ok(AssetProtocol::Bitcoin),
            "taproot_assets" => Ok(AssetProtocol::TaprootAssets),
            other => Err(format!("unsupported asset protocol: {other}")),
        }
    }
}

/// Asset settlement network.
#[derive(Debug, Clone, Copy, Default, Deserialize, Serialize, PartialEq, Eq, ToSchema)]
pub enum AssetNetwork {
    #[default]
    #[serde(rename = "bitcoin/mainnet")]
    BitcoinMainnet,
    #[serde(rename = "bitcoin/testnet")]
    BitcoinTestnet,
    #[serde(rename = "bitcoin/testnet4")]
    BitcoinTestnet4,
    #[serde(rename = "bitcoin/regtest")]
    BitcoinRegtest,
    #[serde(rename = "bitcoin/simnet")]
    BitcoinSimnet,
    #[serde(rename = "bitcoin/signet")]
    BitcoinSignet,
}

impl AssetNetwork {
    pub const fn as_str(self) -> &'static str {
        match self {
            AssetNetwork::BitcoinMainnet => "bitcoin/mainnet",
            AssetNetwork::BitcoinTestnet => "bitcoin/testnet",
            AssetNetwork::BitcoinTestnet4 => "bitcoin/testnet4",
            AssetNetwork::BitcoinRegtest => "bitcoin/regtest",
            AssetNetwork::BitcoinSimnet => "bitcoin/simnet",
            AssetNetwork::BitcoinSignet => "bitcoin/signet",
        }
    }
}

impl From<BtcNetwork> for AssetNetwork {
    fn from(network: BtcNetwork) -> Self {
        match network {
            BtcNetwork::Bitcoin => AssetNetwork::BitcoinMainnet,
            BtcNetwork::Testnet => AssetNetwork::BitcoinTestnet,
            BtcNetwork::Testnet4 => AssetNetwork::BitcoinTestnet4,
            BtcNetwork::Regtest => AssetNetwork::BitcoinRegtest,
            BtcNetwork::Simnet => AssetNetwork::BitcoinSimnet,
            BtcNetwork::Signet => AssetNetwork::BitcoinSignet,
        }
    }
}

impl std::fmt::Display for AssetNetwork {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(self.as_str())
    }
}

impl std::str::FromStr for AssetNetwork {
    type Err = String;

    fn from_str(value: &str) -> Result<Self, Self::Err> {
        match value {
            "bitcoin/mainnet" => Ok(AssetNetwork::BitcoinMainnet),
            "bitcoin/testnet" => Ok(AssetNetwork::BitcoinTestnet),
            "bitcoin/testnet4" => Ok(AssetNetwork::BitcoinTestnet4),
            "bitcoin/regtest" => Ok(AssetNetwork::BitcoinRegtest),
            "bitcoin/simnet" => Ok(AssetNetwork::BitcoinSimnet),
            "bitcoin/signet" => Ok(AssetNetwork::BitcoinSignet),
            other => Err(format!("unsupported asset network: {other}")),
        }
    }
}

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
    /// Settlement protocol.
    #[schema(example = "bitcoin")]
    pub protocol: AssetProtocol,
    /// Settlement network.
    #[schema(example = "bitcoin/mainnet")]
    pub network: AssetNetwork,
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
    /// Owning account ID.
    ///
    /// User-scoped endpoints populate this from the authenticated account.
    pub account_id: Option<Uuid>,
    /// Asset ID
    pub asset_id: Option<Uuid>,
    /// Direction of the ordering of results
    #[serde(default)]
    pub order_direction: OrderDirection,
}
