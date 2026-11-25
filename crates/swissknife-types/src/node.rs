use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use serde_with::{serde_as, DisplayFromStr};
use utoipa::ToSchema;

/// Point-in-time observations from the configured Lightning provider.
#[derive(Debug, Serialize, Deserialize, ToSchema)]
pub struct NodeOverview {
    pub provider: String,
    pub observed_at: DateTime<Utc>,
    pub info: Option<NodeInfo>,
    pub peers: Option<Vec<NodePeer>>,
    pub channels: Option<Vec<NodeChannel>>,
    pub onchain: Option<NodeOnchainBalance>,
    pub liquidity: Option<NodeLiquidity>,
    pub alerts: Vec<NodeAlert>,
}

#[derive(Debug, Serialize, Deserialize, ToSchema)]
pub struct NodeInfo {
    pub alias: String,
    pub network: String,
    pub block_height: u32,
    pub synced_to_chain: bool,
}

#[derive(Debug, Serialize, Deserialize, ToSchema)]
pub struct NodePeer {
    pub id: String,
    pub connected: bool,
}

#[serde_as]
#[derive(Debug, Serialize, Deserialize, ToSchema)]
pub struct NodeChannel {
    /// Provider channel identifier, if already assigned.
    pub id: Option<String>,
    pub peer_id: String,
    pub active: bool,
    /// CLN spendable amount or LND local balance less channel reserve.
    #[serde_as(as = "Option<DisplayFromStr>")]
    #[schema(value_type = Option<String>)]
    pub outbound_msat: Option<u64>,
    /// CLN receivable amount or LND remote balance less channel reserve.
    #[serde_as(as = "Option<DisplayFromStr>")]
    #[schema(value_type = Option<String>)]
    pub inbound_msat: Option<u64>,
}

#[serde_as]
#[derive(Debug, Serialize, Deserialize, ToSchema)]
pub struct NodeLiquidity {
    /// Sum over active channels. A missing provider value makes that direction unknown.
    #[serde_as(as = "Option<DisplayFromStr>")]
    #[schema(value_type = Option<String>)]
    pub outbound_msat: Option<u64>,
    #[serde_as(as = "Option<DisplayFromStr>")]
    #[schema(value_type = Option<String>)]
    pub inbound_msat: Option<u64>,
}

#[derive(Debug, Serialize, Deserialize, ToSchema)]
pub struct NodeOnchainBalance {
    /// Confirmed provider wallet funds, including reserved outputs.
    pub confirmed_sat: u64,
    pub unconfirmed_sat: u64,
}

#[derive(Debug, PartialEq, Eq, Serialize, Deserialize, ToSchema)]
#[serde(rename_all = "snake_case")]
pub enum NodeAlert {
    InfoUnavailable,
    PeersUnavailable,
    ChannelsUnavailable,
    OnchainUnavailable,
    ChainNotSynced,
    NoConnectedPeers,
    NoActiveChannels,
    LiquidityUnavailable,
}
