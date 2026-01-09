mod application;
mod domains;
mod infra;

use std::process::exit;
use std::sync::Arc;

#[cfg(debug_assertions)]
use dotenv::dotenv;
use tokio::signal::ctrl_c;
use tokio::signal::unix::signal;
use tokio::signal::unix::SignalKind;
use tracing::debug;
use tracing::info;

use crate::infra::app::AppState;
use crate::infra::app::Server;
use crate::infra::config::config_rs::load_config;
use crate::infra::logging::tracing::setup_tracing;

use tracing::error;

#[tokio::main]
async fn main() {
    // Load .env file in development
    #[cfg(debug_assertions)]
    dotenv().ok();

    // Load config and logger
    let config = match load_config() {
        Ok(c) => c,
        Err(err) => {
            println!("Failed to load config: {:?}", err);
            exit(1);
        }
    };
    setup_tracing(config.logging.clone());

    let app_state = match AppState::new(config.clone()).await {
        Ok(state) => Arc::new(state),
        Err(err) => {
            error!(%err, "failed to create app state");
            exit(1);
        }
    };

    if let Err(err) = app_state.services.invoice.sync().await {
        error!(%err, "failed to sync invoices");
        exit(1);
    }

    if let Err(err) = app_state.bitcoin_events.sync_pending_transactions().await {
        error!(%err, "failed to sync onchain transactions");
        exit(1);
    }

    let app = Server::new(app_state.clone(), config.dashboard_dir.as_deref());
    if let Err(err) = app.start(&config.web.addr, shutdown_signal(app_state.clone())).await {
        error!(%err, "failed to start API server");
        exit(1);
    }
}

async fn shutdown_signal(state: Arc<AppState>) {
    let ctrl_c = async {
        if let Err(err) = ctrl_c().await {
            error!(%err, "Failed to install Ctrl+C handler");
        }
        info!("Received Ctrl+C signal. Shutting down gracefully");
    };

    #[cfg(unix)]
    let terminate = async {
        match signal(SignalKind::terminate()) {
            Ok(mut stream) => {
                stream.recv().await;
                debug!("Received SIGTERM. Shutting down gracefully");
            }
            Err(err) => error!(%err, "Failed to install SIGTERM handler"),
        }
    };

    #[cfg(not(unix))]
    let terminate = std::future::pending::<()>();

    tokio::select! {
        _ = ctrl_c => {},
        _ = terminate => {},
    }

    if let Err(err) = state.ln_client.disconnect().await {
        error!(%err, "Failed to shutdown gracefully");
    }

    info!("Shutdown complete");
}
