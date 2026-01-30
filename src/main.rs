mod application;
mod domains;
mod infra;

use std::{process::exit, sync::Arc};

#[cfg(debug_assertions)]
use dotenv::dotenv;
use tokio::signal::{
    ctrl_c,
    unix::{signal, SignalKind},
};
use tracing::{debug, error, info};

use crate::application::entities::{AppAdapters, AppServices};
use crate::infra::{
    app::{EventListener, Server},
    config::config_rs::load_config,
    logging::tracing::setup_tracing,
};

#[tokio::main]
async fn main() {
    // Load .env file in development
    #[cfg(debug_assertions)]
    dotenv().ok();

    info!("Numeraire SwissKnife version: {}", env!("CARGO_PKG_VERSION"));

    // Load config and logger
    let config = match load_config() {
        Ok(c) => c,
        Err(err) => {
            println!("Failed to load config: {:?}", err);
            exit(1);
        }
    };
    setup_tracing(config.logging.clone());

    let adapters = match AppAdapters::new(config.clone()).await {
        Ok(state) => state,
        Err(err) => {
            error!(%err, "failed to create app state");
            exit(1);
        }
    };

    let services = Arc::new(AppServices::new(config.clone(), adapters.clone()));

    let event_listener = match EventListener::new(config.clone(), adapters.clone(), services.clone()).await {
        Ok(listener) => listener,
        Err(err) => {
            error!(%err, "failed to build event listener");
            exit(1);
        }
    };

    // Start the event listener first so we don't miss any events.
    if let Err(err) = event_listener.start().await {
        error!(%err, "failed to start event listener");
        exit(1);
    }

    // We start accepting external requests only when everything is synced and ready
    let app = Server::new(adapters.clone(), services.clone(), config.dashboard_dir.as_deref());
    if let Err(err) = app.start(&config.web.addr, shutdown_signal(adapters.clone())).await {
        error!(%err, "failed to start API server");
        exit(1);
    }
}

async fn shutdown_signal(adapters: AppAdapters) {
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

    if let Err(err) = adapters.ln_client.disconnect().await {
        error!(%err, "Failed to shutdown gracefully");
    }

    info!("Shutdown complete");
}
