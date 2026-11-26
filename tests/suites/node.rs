use reqwest::StatusCode;
use swissknife_types::{NodeOverview, Permission};

use crate::common::{app, assert_status, Auth};

#[tokio::test]
async fn scoped_read_reports_live_regtest_peers_channels_and_balances() {
    let app = app().await;
    let admin = app.admin_token().await;
    let api = app.api();
    let path = "/v1/node/overview";
    assert_status(&api.get(path, Auth::None).await, StatusCode::UNAUTHORIZED);
    let wallet_reader = app.api_key(admin, vec![Permission::ReadWallet]).await;
    assert_status(
        &api.get(path, Auth::ApiKey(&wallet_reader)).await,
        StatusCode::FORBIDDEN,
    );

    let wallet = app.create_wallet(admin, "node-health-deposit").await;
    let deposit_sat = 15_000;
    app.fund_onchain(admin, wallet.id, deposit_sat).await;

    let node_reader = app.api_key(admin, vec![Permission::ReadLnNode]).await;
    let response = api.get(path, Auth::ApiKey(&node_reader)).await;
    assert_status(&response, StatusCode::OK);
    let node = response.parse::<NodeOverview>();
    assert!(node.alerts.is_empty(), "{node:?}");
    let info = node.info.unwrap();
    assert_eq!(info.network, "regtest");
    assert!(info.block_height > 0);
    let peers = node.peers.unwrap();
    let channels = node.channels.unwrap();
    let active = channels
        .iter()
        .find(|channel| channel.active)
        .expect("funded regtest channel");
    assert!(peers.iter().any(|peer| peer.connected && peer.id == active.peer_id));
    let liquidity = node.liquidity.unwrap();
    assert!(liquidity.outbound_msat.unwrap() + liquidity.inbound_msat.unwrap() > 0);
    assert!(node.onchain.unwrap().confirmed_sat >= deposit_sat);
    assert!(response.body["liquidity"]["outbound_msat"].is_string());
}
