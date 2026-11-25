mod http;
mod resources;
mod tools;
mod wallet;
mod withdraw;

use anyhow::Context;
use rmcp::{transport::stdio, ServiceExt};
use std::net::SocketAddr;
use uuid::Uuid;

/// Serve one wallet over stdin/stdout using the owner's delegated credential.
pub async fn run(
    server: String,
    wallet_id: Uuid,
    listen: Option<SocketAddr>,
    public_origin: Option<&str>,
    withdraw_origins: &[String],
) -> anyhow::Result<()> {
    let secret = std::env::var("SWISSKNIFE_MCP_WALLET_KEY")
        .context("Set SWISSKNIFE_MCP_WALLET_KEY to a delegated wallet credential")?;
    let client = wallet::WalletClient::new(&server, wallet_id, &secret)?;
    let withdrawals = withdraw::WithdrawalClient::new(withdraw_origins)?;
    if let Some(listen) = listen {
        return http::serve(client, withdrawals, listen, public_origin).await;
    }
    let service = tools::WalletServer::new(client, resources::ResourceFeeds::default(), withdrawals)
        .serve(stdio())
        .await?;
    service.waiting().await?;
    Ok(())
}
