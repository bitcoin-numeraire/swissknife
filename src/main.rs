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
use tracing::error;

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
            error!(error = ?e);
            exit(1);
        }
    };

    let app = App::new(app_state);
    if let Err(e) = app.start(&config.web.addr).await {
        error!(error = ?e);
        exit(1);
    }
}
