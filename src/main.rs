mod application;
mod domains;
mod infra;

use std::process::exit;
use std::sync::Arc;

#[cfg(debug_assertions)]
use dotenv::dotenv;
use infra::app::start;

use crate::infra::app::AppState;
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
        Ok(app_state) => app_state,
        Err(err) => {
            error!(%err, "failed to create app state");
            exit(1);
        }
    };

    if let Err(err) = start(Arc::new(app_state), &config.web.addr).await {
        error!(%err, "failed to start app");
        exit(1);
    }
}
