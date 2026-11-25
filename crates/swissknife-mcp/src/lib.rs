mod tools;
mod wallet;

use anyhow::Context;
use rmcp::{transport::stdio, ServiceExt};
use uuid::Uuid;

/// Serve one wallet over stdin/stdout using the owner's delegated credential.
pub async fn run(server: String, wallet_id: Uuid) -> anyhow::Result<()> {
    let secret = std::env::var("SWISSKNIFE_MCP_WALLET_KEY")
        .context("Set SWISSKNIFE_MCP_WALLET_KEY to a delegated wallet credential")?;
    let client = wallet::WalletClient::new(&server, wallet_id, &secret)?;
    let service = tools::WalletServer::new(client).serve(stdio()).await?;
    service.waiting().await?;
    Ok(())
}
