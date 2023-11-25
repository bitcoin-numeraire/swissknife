mod adapters;
mod application;
mod domains;

use std::sync::Arc;

use adapters::rgb::rgblib::{RGBLibClient, RGBLibClientConfig};
use adapters::rgb::DynRGBClient;
use adapters::web::axum::{AxumServer, AxumServerConfig};
use domains::rgb::api::http::RGBHandler;

use crate::adapters::web::WebServer;

#[tokio::main]
async fn main() {
    let addr = "0.0.0.0:3000";

    // Create app
    let server = AxumServer::new(AxumServerConfig {
        addr: addr.to_string(),
    })
    .unwrap();

    let config = RGBLibClientConfig {
        electrum_url: "localhost:50001".to_string(),
        data_dir: "storage".to_string(),
        mnemonic:
            "adapt lumber inherit square defy burden beyond assault drop lumber purpose satoshi"
                .to_string(),
    };

    let rgb_client = RGBLibClient::new(config.clone()).await.unwrap();

    println!(
        "Wallet created in directory `{}` with mnemonic: `{}`",
        config.data_dir, config.mnemonic
    );

    let rgb_client = Arc::new(rgb_client) as DynRGBClient;

    server
        .nest_router("/rgb", RGBHandler::new().routes(rgb_client))
        .await
        .unwrap();

    // Start server
    let server_future = server.start();
    let ctrl_c_future = tokio::signal::ctrl_c();

    println!("Listening on {}", addr);

    tokio::select! {
        result = server_future => {
            if let Err(e) = result {
                eprintln!("Server error: {}", e);
            }
        }
        _ = ctrl_c_future => {
            println!("Received Ctrl+C, shutting down");
        }
    }
}
