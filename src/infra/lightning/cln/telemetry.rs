use anyhow::{Context, Result};
use async_trait::async_trait;
use serde::Deserialize;
use swissknife_types::{NodeChannel, NodeInfo, NodeOnchainBalance, NodePeer};

use super::{cln, ClnGrpcClient, ClnRestClient, ListFundsRequest, ListFundsResponse};
use crate::domains::node::NodeReader;

#[derive(Deserialize)]
struct Info {
    alias: String,
    network: String,
    blockheight: u32,
    warning_bitcoind_sync: Option<String>,
    warning_lightningd_sync: Option<String>,
}

impl From<Info> for NodeInfo {
    fn from(info: Info) -> Self {
        Self {
            alias: info.alias,
            network: info.network,
            block_height: info.blockheight,
            synced_to_chain: info.warning_bitcoind_sync.is_none() && info.warning_lightningd_sync.is_none(),
        }
    }
}

#[derive(Deserialize)]
struct Peers {
    peers: Vec<NodePeer>,
}

#[derive(Deserialize)]
struct Channels {
    channels: Vec<PeerChannel>,
}

#[derive(Deserialize)]
struct PeerChannel {
    channel_id: Option<String>,
    peer_id: String,
    peer_connected: bool,
    state: String,
    spendable_msat: Option<u64>,
    receivable_msat: Option<u64>,
}

fn balance(outputs: impl Iterator<Item = (bool, Option<u64>)>) -> Result<NodeOnchainBalance> {
    let mut result = NodeOnchainBalance {
        confirmed_sat: 0,
        unconfirmed_sat: 0,
    };
    for (confirmed, amount) in outputs {
        let amount = amount.context("Missing on-chain output amount")?;
        let sum = if confirmed {
            &mut result.confirmed_sat
        } else {
            &mut result.unconfirmed_sat
        };
        *sum = sum.checked_add(amount / 1000).context("Invalid on-chain total")?;
    }
    Ok(result)
}

#[async_trait]
impl NodeReader for ClnRestClient {
    async fn info(&self) -> Result<NodeInfo> {
        Ok(self
            .post_request::<Info>("getinfo", &serde_json::json!({}))
            .await?
            .into())
    }

    async fn peers(&self) -> Result<Vec<NodePeer>> {
        Ok(self
            .post_request::<Peers>("listpeers", &serde_json::json!({}))
            .await?
            .peers)
    }

    async fn channels(&self) -> Result<Vec<NodeChannel>> {
        let response: Channels = self.post_request("listpeerchannels", &serde_json::json!({})).await?;
        Ok(response
            .channels
            .into_iter()
            .map(|ch| NodeChannel {
                id: ch.channel_id,
                peer_id: ch.peer_id,
                active: ch.peer_connected && ch.state == "CHANNELD_NORMAL",
                outbound_msat: ch.spendable_msat,
                inbound_msat: ch.receivable_msat,
            })
            .collect())
    }

    async fn onchain(&self) -> Result<NodeOnchainBalance> {
        let funds: ListFundsResponse = self
            .post_request("listfunds", &ListFundsRequest { spent: Some(false) })
            .await?;
        balance(funds.outputs.into_iter().filter_map(|o| match o.status.as_str() {
            "confirmed" => Some((true, Some(o.amount_msat))),
            "unconfirmed" => Some((false, Some(o.amount_msat))),
            _ => None,
        }))
    }
}

#[async_trait]
impl NodeReader for ClnGrpcClient {
    async fn info(&self) -> Result<NodeInfo> {
        let info = self.client.clone().getinfo(cln::GetinfoRequest {}).await?.into_inner();
        Ok(Info {
            alias: info.alias,
            network: info.network,
            blockheight: info.blockheight,
            warning_bitcoind_sync: info.warning_bitcoind_sync,
            warning_lightningd_sync: info.warning_lightningd_sync,
        }
        .into())
    }

    async fn peers(&self) -> Result<Vec<NodePeer>> {
        let peers = self
            .client
            .clone()
            .list_peers(cln::ListpeersRequest::default())
            .await?
            .into_inner();
        Ok(peers
            .peers
            .into_iter()
            .map(|p| NodePeer {
                id: hex::encode(p.id),
                connected: p.connected,
            })
            .collect())
    }

    async fn channels(&self) -> Result<Vec<NodeChannel>> {
        let channels = self
            .client
            .clone()
            .list_peer_channels(cln::ListpeerchannelsRequest::default())
            .await?
            .into_inner();
        Ok(channels
            .channels
            .into_iter()
            .map(|ch| NodeChannel {
                id: ch.channel_id.map(hex::encode),
                peer_id: hex::encode(ch.peer_id),
                active: ch.peer_connected && ch.state == cln::ChannelState::ChanneldNormal as i32,
                outbound_msat: ch.spendable_msat.map(|v| v.msat),
                inbound_msat: ch.receivable_msat.map(|v| v.msat),
            })
            .collect())
    }

    async fn onchain(&self) -> Result<NodeOnchainBalance> {
        use cln::listfunds_outputs::ListfundsOutputsStatus;
        let funds = self
            .client
            .clone()
            .list_funds(cln::ListfundsRequest { spent: Some(false) })
            .await?
            .into_inner();
        balance(
            funds
                .outputs
                .into_iter()
                .filter_map(|o| match ListfundsOutputsStatus::try_from(o.status) {
                    Ok(ListfundsOutputsStatus::Confirmed) => Some((true, o.amount_msat.map(|a| a.msat))),
                    Ok(ListfundsOutputsStatus::Unconfirmed) => Some((false, o.amount_msat.map(|a| a.msat))),
                    _ => None,
                }),
        )
    }
}
