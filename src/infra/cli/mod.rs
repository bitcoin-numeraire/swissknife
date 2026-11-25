use clap::{Parser, Subcommand};
use uuid::Uuid;

use crate::{application::errors::ApplicationError, domains::account::LoginRecoveryUseCases};

/// Start the SwissKnife server or run an operator command.
#[derive(Parser)]
#[command(version)]
pub struct Cli {
    #[command(subcommand)]
    pub command: Option<Command>,
}

#[derive(Subcommand)]
pub enum Command {
    /// Serve a single remote or local wallet over MCP stdio or Streamable HTTP. Uses SWISSKNIFE_MCP_WALLET_KEY.
    Mcp {
        /// HTTPS origin of the SwissKnife API, or HTTP on loopback.
        #[arg(long)]
        server: String,
        /// Wallet UUID bound to the delegated credential.
        #[arg(long)]
        wallet: Uuid,
        /// Serve /mcp on this loopback socket. Requires SWISSKNIFE_MCP_HTTP_TOKEN.
        #[arg(long)]
        http_listen: Option<std::net::SocketAddr>,
        /// HTTPS origin of an optional TLS proxy that preserves the public Host header.
        #[arg(long, requires = "http_listen")]
        public_origin: Option<String>,
        /// Trusted LNURL-withdraw service origin. Repeat for additional discovery or callback origins.
        #[arg(long = "withdraw-origin", value_name = "ORIGIN")]
        withdraw_origins: Vec<String>,
    },
    /// Enable an existing local login and issue a one-time password reset code.
    /// Uses the configured database without connecting to a Lightning provider.
    RecoverLocalLogin {
        /// Account UUID whose existing login should be recovered.
        account_id: Uuid,
    },
}

pub async fn recover_local_login(
    account_id: Uuid,
    recovery: &dyn LoginRecoveryUseCases,
) -> Result<(), ApplicationError> {
    let grant = recovery.recover(account_id).await?;
    println!("Local login enabled. Existing sessions and password revoked.");
    println!(
        "Use this one-time code on this instance's /reset-password page before {}:",
        grant.expires_at
    );
    println!("{}", grant.code);
    Ok(())
}
