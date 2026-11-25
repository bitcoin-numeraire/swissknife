use std::{future::Future, sync::Arc, time::Duration};

use async_trait::async_trait;
use chrono::Utc;
use swissknife_types::{NodeAlert, NodeChannel, NodeInfo, NodeLiquidity, NodeOnchainBalance, NodeOverview, NodePeer};

#[cfg_attr(test, mockall::automock)]
#[async_trait]
pub trait NodeReader: Send + Sync {
    async fn info(&self) -> anyhow::Result<NodeInfo>;
    async fn peers(&self) -> anyhow::Result<Vec<NodePeer>>;
    async fn channels(&self) -> anyhow::Result<Vec<NodeChannel>>;
    async fn onchain(&self) -> anyhow::Result<NodeOnchainBalance>;
}

pub struct NodeService {
    reader: Arc<dyn NodeReader>,
    provider: String,
    timeout: Duration,
}

impl NodeService {
    pub fn new(reader: Arc<dyn NodeReader>, provider: String, timeout: Duration) -> Self {
        Self {
            reader,
            provider,
            timeout,
        }
    }

    async fn observe<T>(&self, section: &'static str, read: impl Future<Output = anyhow::Result<T>>) -> Option<T> {
        match tokio::time::timeout(self.timeout, read).await {
            Ok(Ok(data)) => Some(data),
            result => {
                tracing::warn!(section, provider = %self.provider, timed_out = result.is_err(), "Node telemetry unavailable");
                None
            }
        }
    }

    pub async fn overview(&self) -> NodeOverview {
        let (info, peers, channels, onchain) = tokio::join!(
            self.observe("info", self.reader.info()),
            self.observe("peers", self.reader.peers()),
            self.observe("channels", self.reader.channels()),
            self.observe("onchain", self.reader.onchain()),
        );
        let mut alerts = Vec::new();
        match &info {
            None => alerts.push(NodeAlert::InfoUnavailable),
            Some(info) if !info.synced_to_chain => alerts.push(NodeAlert::ChainNotSynced),
            _ => {}
        }
        match &peers {
            None => alerts.push(NodeAlert::PeersUnavailable),
            Some(peers) if !peers.iter().any(|peer| peer.connected) => alerts.push(NodeAlert::NoConnectedPeers),
            _ => {}
        }
        let liquidity = channels.as_ref().map(|channels| {
            let active = channels.iter().filter(|channel| channel.active);
            if active.clone().next().is_none() {
                alerts.push(NodeAlert::NoActiveChannels);
            }
            let outbound_msat = active
                .clone()
                .try_fold(0_u64, |total, c| total.checked_add(c.outbound_msat?));
            let inbound_msat = active
                .clone()
                .try_fold(0_u64, |total, c| total.checked_add(c.inbound_msat?));
            if outbound_msat.is_none() || inbound_msat.is_none() {
                alerts.push(NodeAlert::LiquidityUnavailable);
            }
            NodeLiquidity {
                outbound_msat,
                inbound_msat,
            }
        });
        if channels.is_none() {
            alerts.push(NodeAlert::ChannelsUnavailable);
        }
        if onchain.is_none() {
            alerts.push(NodeAlert::OnchainUnavailable);
        }
        NodeOverview {
            provider: self.provider.clone(),
            observed_at: Utc::now(),
            info,
            peers,
            channels,
            onchain,
            liquidity,
            alerts,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn unavailable_sections_preserve_independent_observations() {
        let mut reader = MockNodeReader::new();
        reader.expect_info().returning(|| anyhow::bail!("provider detail"));
        reader.expect_onchain().returning(|| {
            Ok(NodeOnchainBalance {
                confirmed_sat: 21,
                unconfirmed_sat: 0,
            })
        });
        reader.expect_peers().returning(|| {
            Ok(vec![NodePeer {
                id: "peer".into(),
                connected: false,
            }])
        });
        reader.expect_channels().returning(|| {
            Ok(vec![
                NodeChannel {
                    id: None,
                    peer_id: "a".into(),
                    active: true,
                    outbound_msat: Some(42),
                    inbound_msat: None,
                },
                NodeChannel {
                    id: None,
                    peer_id: "b".into(),
                    active: false,
                    outbound_msat: Some(123),
                    inbound_msat: Some(999),
                },
            ])
        });
        let node = NodeService::new(Arc::new(reader), "cln_rest".into(), Duration::from_secs(1))
            .overview()
            .await;
        assert!(node.info.is_none());
        assert_eq!(node.onchain.unwrap().confirmed_sat, 21);
        let liquidity = node.liquidity.unwrap();
        assert_eq!(liquidity.outbound_msat, Some(42));
        assert_eq!(liquidity.inbound_msat, None);
        assert_eq!(
            node.alerts,
            vec![
                NodeAlert::InfoUnavailable,
                NodeAlert::NoConnectedPeers,
                NodeAlert::LiquidityUnavailable
            ]
        );
    }
}
