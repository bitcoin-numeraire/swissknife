mod http;
mod pass_status;
mod resources;
mod tools;
mod wallet;
mod withdraw;

use anyhow::Context;
use rmcp::{transport::stdio, ServiceExt};
use std::{
    net::{IpAddr, SocketAddr},
    time::Duration,
};
use uuid::Uuid;

fn secure_service_url(url: &reqwest::Url) -> bool {
    let loopback = url
        .host_str()
        .and_then(|host| host.trim_matches(['[', ']']).parse::<IpAddr>().ok())
        .is_some_and(|ip| ip.is_loopback());
    (url.scheme() == "https" || (url.scheme() == "http" && loopback))
        && url.has_host()
        && url.username().is_empty()
        && url.password().is_none()
}

fn external_client() -> reqwest::Result<reqwest::Client> {
    reqwest::Client::builder()
        .redirect(reqwest::redirect::Policy::none())
        .retry(reqwest::retry::never())
        .timeout(Duration::from_secs(15))
        .build()
}

/// Serve one wallet over stdin/stdout using the owner's delegated credential.
pub async fn run(
    server: String,
    wallet_id: Uuid,
    listen: Option<SocketAddr>,
    public_origin: Option<&str>,
    withdraw_origins: &[String],
    merchant_resources: &[String],
) -> anyhow::Result<()> {
    let secret = std::env::var("SWISSKNIFE_MCP_WALLET_KEY")
        .context("Set SWISSKNIFE_MCP_WALLET_KEY to a delegated wallet credential")?;
    let client = wallet::WalletClient::new(&server, wallet_id, &secret)?;
    let withdrawals = withdraw::WithdrawalClient::new(withdraw_origins)?;
    let passes = pass_status::PassClient::new(merchant_resources)?;
    if let Some(listen) = listen {
        return http::serve(client, withdrawals, passes, listen, public_origin).await;
    }
    let service = tools::WalletServer::new(client, resources::ResourceFeeds::default(), withdrawals, passes)
        .serve(stdio())
        .await?;
    service.waiting().await?;
    Ok(())
}
