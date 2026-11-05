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
    /// Enable an existing local login and issue a one-time password reset code.
    /// Uses the configured database without connecting to a Lightning provider.
    RecoverLocalLogin {
        /// Account UUID whose existing login should be recovered.
        account_id: Uuid,
    },
}

impl Command {
    pub async fn run(self, recovery: &dyn LoginRecoveryUseCases) -> Result<(), ApplicationError> {
        match self {
            Self::RecoverLocalLogin { account_id } => {
                let grant = recovery.recover(account_id).await?;
                println!("Local login enabled. Existing sessions and password revoked.");
                println!(
                    "Use this one-time code on this instance's /reset-password page before {}:",
                    grant.expires_at
                );
                println!("{}", grant.code);
                Ok(())
            }
        }
    }
}
