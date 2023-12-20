mod adapters;
mod application;
mod domains;

use crate::adapters::config::config_rs::ConfigRsLoader;
use crate::adapters::config::ConfigLoader;
use adapters::app::App;
use tracing::{debug, error, info};

#[tokio::main]
async fn main() {
    // Load config and logger
    let config = ConfigRsLoader {}.load().unwrap();
    debug!(?config, "Loaded configuration");

    let app = App::new(config.clone()).await;

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
