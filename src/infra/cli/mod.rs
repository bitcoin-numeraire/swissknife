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
    /// Serve a single remote or local wallet over MCP stdio. Uses SWISSKNIFE_MCP_WALLET_KEY.
    Mcp {
        /// HTTPS origin of the SwissKnife API, or HTTP on loopback.
        #[arg(long)]
        server: String,
        /// Wallet UUID bound to the delegated credential.
        #[arg(long)]
        wallet: Uuid,
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
