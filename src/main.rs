mod application;
mod domains;
mod infra;

use std::process::exit;

#[cfg(debug_assertions)]
use dotenv::dotenv;

use crate::infra::app::App;
use crate::infra::app::AppState;
use crate::infra::config::config_rs::load_config;
use crate::infra::logging::tracing::setup_tracing;

use tracing::error;

#[tokio::main]
async fn main() {
    // Load .env file in development
    // TODO: Remove this in production
    #[cfg(debug_assertions)]
    dotenv().ok();

    // Load config and logger
    let config = match load_config() {
        Ok(c) => c,
        Err(e) => {
            panic!("Error loading config: {:?}", e);
        }
    };
    setup_tracing(config.logging.clone());

    let app_state = match AppState::new(config.clone()).await {
        Ok(app_state) => app_state,
        Err(e) => {
            error!(error = e.to_string(), "failed to create app state");
            exit(1);
        }
    };

    let app = App::new(app_state);
    if let Err(e) = app.start(&config.web.addr).await {
        error!(error = ?e);
        exit(1);
    }
}
