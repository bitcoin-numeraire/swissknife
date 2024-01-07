mod adapters;
mod application;
mod domains;

use std::process::exit;

#[cfg(debug_assertions)]
use dotenv::dotenv;

use crate::adapters::config::config_rs::load_config;
use crate::adapters::logging::tracing::setup_tracing;

use adapters::app::App;
use adapters::app::AppState;
use tracing::{error, info};

#[tokio::main]
async fn main() {
    // Load .env file in development
    // TODO: Remove this in production
    #[cfg(debug_assertions)]
    dotenv().ok();

    // Load config and logger
    let config = match load_config() {
        Ok(app_state) => app_state,
        Err(e) => {
            error!(error = ?e);
            exit(1);
        }
    };
    setup_tracing(config.logging.clone());

    let app_state = match AppState::new(config.clone()).await {
        Ok(app_state) => app_state,
        Err(e) => {
            error!(error = ?e, "Failed to initialize application");
            exit(1);
        }
    };

    let app = App::new(app_state);

    let server_future = app.start(&config.web.addr);
    let ctrl_c_future = tokio::signal::ctrl_c();

    tokio::select! {
        result = server_future => {
            if let Err(e) = result {
                error!(error = ?e, "Server error");
            }
        }
        _ = ctrl_c_future => {
            info!("Received Ctrl+C, shutting down");
        }
    }
}
