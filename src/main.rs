mod adapters;
mod application;
mod domains;

use crate::adapters::config::config_rs::ConfigRsLoader;
use crate::adapters::config::ConfigLoader;
use crate::adapters::logging::tracing::setup_tracing;
use adapters::app::App;
use adapters::app::AppState;
use tracing::{debug, error, info};

#[tokio::main]
async fn main() {
    // Load config and logger
    let config = ConfigRsLoader {}.load().unwrap();
    setup_tracing(config.logging.clone());
    debug!(?config, "Loaded configuration"); // TODO: remove this line

    let app_state = AppState::new(config.clone()).await.unwrap();
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
