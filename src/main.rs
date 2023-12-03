mod adapters;
mod application;
mod domains;

use std::sync::Arc;

use adapters::config::config_rs::ConfigRsLoader;
use adapters::config::ConfigLoader;
use adapters::lightning::breez::BreezClient;
use adapters::lightning::DynLightningClient;
use adapters::logging::tracing::setup_tracing;
use adapters::rgb::rgblib::RGBLibClient;
use adapters::rgb::DynRGBClient;
use adapters::web::axum::AxumServer;
use adapters::web::WebServer;
use domains::lightning::api::http::LightningHandler;
use domains::rgb::api::http::RGBHandler;
use tracing::{error, info};

#[tokio::main]
async fn main() {
    // Load config
    let config: application::dtos::AppConfig = ConfigRsLoader {}.load().unwrap();

    // Create adapters
    setup_tracing(config.logging.clone());
    let mut server = AxumServer::new(config.web.clone()).unwrap();
    let rgb_client = RGBLibClient::new(config.rgb.clone()).await.unwrap();
    let lightning_client = BreezClient::new(config.lightning.clone()).await.unwrap();

    server
        .nest_router(
            "/rgb",
            RGBHandler::routes(Arc::new(rgb_client) as DynRGBClient),
        )
        .await
        .nest_router("/.well-known", LightningHandler::well_known_routes())
        .await
        .nest_router(
            "/lightning",
            LightningHandler::routes(Arc::new(lightning_client) as DynLightningClient),
        )
        .await;

    // Start server
    let server_future = server.start();
    let ctrl_c_future = tokio::signal::ctrl_c();

    info!(addr = %config.web.addr, "Listening on");

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
