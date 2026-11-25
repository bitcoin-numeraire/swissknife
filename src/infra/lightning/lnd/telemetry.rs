use anyhow::{Context, Result};
use async_trait::async_trait;
use serde::Deserialize;
use serde_with::{serde_as, DisplayFromStr};
use swissknife_types::{NodeChannel, NodeInfo, NodeOnchainBalance, NodePeer};

use super::{lnd_types::Chain, lnrpc, LndGrpcClient, LndRestClient};
use crate::domains::node::NodeReader;

#[derive(Deserialize)]
struct Info {
    alias: String,
    chains: Vec<Chain>,
    block_height: u32,
    synced_to_chain: bool,
}

#[derive(Deserialize)]
struct Peers {
    peers: Vec<Peer>,
}

#[derive(Deserialize)]
struct Peer {
    pub_key: String,
}

#[derive(Deserialize)]
struct Channels {
    channels: Vec<Channel>,
}

#[serde_as]
#[derive(Deserialize)]
struct Channel {
    channel_point: String,
    active: bool,
    remote_pubkey: String,
    #[serde_as(as = "DisplayFromStr")]
    local_balance: i64,
    #[serde_as(as = "DisplayFromStr")]
    remote_balance: i64,
    local_constraints: Option<Reserve>,
    remote_constraints: Option<Reserve>,
}

#[serde_as]
#[derive(Deserialize)]
struct Reserve {
    #[serde_as(as = "DisplayFromStr")]
    chan_reserve_sat: u64,
}

#[serde_as]
#[derive(Deserialize)]
struct WalletBalance {
    #[serde_as(as = "DisplayFromStr")]
    confirmed_balance: u64,
    #[serde_as(as = "DisplayFromStr")]
    unconfirmed_balance: u64,
}

fn available(balance_sat: i64, reserve: Option<u64>) -> Option<u64> {
    u64::try_from(balance_sat)
        .ok()?
        .saturating_sub(reserve?)
        .checked_mul(1000)
}

#[async_trait]
impl NodeReader for LndRestClient {
    async fn info(&self) -> Result<NodeInfo> {
        let info: Info = self.get_request("v1/getinfo").await?;
        Ok(NodeInfo {
            alias: info.alias,
            network: info
                .chains
                .into_iter()
                .find_map(|c| c.network)
                .context("Missing provider network")?,
            block_height: info.block_height,
            synced_to_chain: info.synced_to_chain,
        })
    }

    async fn peers(&self) -> Result<Vec<NodePeer>> {
        let peers: Peers = self.get_request("v1/peers").await?;
        Ok(peers
            .peers
            .into_iter()
            .map(|p| NodePeer {
                id: p.pub_key,
                connected: true,
            })
            .collect())
    }

    async fn channels(&self) -> Result<Vec<NodeChannel>> {
        let channels: Channels = self.get_request("v1/channels").await?;
        Ok(channels
            .channels
            .into_iter()
            .map(|c| NodeChannel {
                id: Some(c.channel_point),
                peer_id: c.remote_pubkey,
                active: c.active,
                outbound_msat: available(c.local_balance, c.local_constraints.map(|v| v.chan_reserve_sat)),
                inbound_msat: available(c.remote_balance, c.remote_constraints.map(|v| v.chan_reserve_sat)),
            })
            .collect())
    }

    async fn onchain(&self) -> Result<NodeOnchainBalance> {
        let funds: WalletBalance = self.get_request("v1/balance/blockchain").await?;
        Ok(NodeOnchainBalance {
            confirmed_sat: funds.confirmed_balance,
            unconfirmed_sat: funds.unconfirmed_balance,
        })
    }
}

#[async_trait]
impl NodeReader for LndGrpcClient {
    async fn info(&self) -> Result<NodeInfo> {
        let info = self
            .client
            .clone()
            .get_info(lnrpc::GetInfoRequest {})
            .await?
            .into_inner();
        Ok(NodeInfo {
            alias: info.alias,
            network: info
                .chains
                .into_iter()
                .next()
                .context("Missing provider network")?
                .network,
            block_height: info.block_height,
            synced_to_chain: info.synced_to_chain,
        })
    }

    async fn peers(&self) -> Result<Vec<NodePeer>> {
        let peers = self
            .client
            .clone()
            .list_peers(lnrpc::ListPeersRequest::default())
            .await?
            .into_inner();
        Ok(peers
            .peers
            .into_iter()
            .map(|p| NodePeer {
                id: p.pub_key,
                connected: true,
            })
            .collect())
    }

    async fn channels(&self) -> Result<Vec<NodeChannel>> {
        let channels = self
            .client
            .clone()
            .list_channels(lnrpc::ListChannelsRequest::default())
            .await?
            .into_inner();
        Ok(channels
            .channels
            .into_iter()
            .map(|c| NodeChannel {
                id: Some(c.channel_point),
                peer_id: c.remote_pubkey,
                active: c.active,
                outbound_msat: available(c.local_balance, c.local_constraints.map(|v| v.chan_reserve_sat)),
                inbound_msat: available(c.remote_balance, c.remote_constraints.map(|v| v.chan_reserve_sat)),
            })
            .collect())
    }

    async fn onchain(&self) -> Result<NodeOnchainBalance> {
        let funds = self
            .client
            .clone()
            .wallet_balance(lnrpc::WalletBalanceRequest::default())
            .await?
            .into_inner();
        Ok(NodeOnchainBalance {
            confirmed_sat: funds.confirmed_balance.try_into()?,
            unconfirmed_sat: funds.unconfirmed_balance.try_into()?,
        })
    }
}
