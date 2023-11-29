mod adapters;
mod application;
mod domains;

use std::sync::Arc;

use adapters::lightning::breez::{BreezClient, BreezClientConfig};
use adapters::lightning::DynLightningClient;
use adapters::rgb::rgblib::{RGBLibClient, RGBLibClientConfig};
use adapters::rgb::DynRGBClient;
use adapters::web::axum::{AxumServer, AxumServerConfig};
use adapters::web::WebServer;
use domains::lightning::api::http::LightningHandler;
use domains::rgb::api::http::RGBHandler;

#[tokio::main]
async fn main() {
    let addr = "0.0.0.0:80";

    // Create app
    let server = AxumServer::new(AxumServerConfig {
        addr: addr.to_string(),
    })
    .unwrap();

    let rgb_lib_config = RGBLibClientConfig {
        electrum_url: "localhost:50001".to_string(),
        data_dir: "storage/rgblib".to_string(),
        mnemonic:
            "adapt lumber inherit square defy burden beyond assault drop lumber purpose satoshi"
                .to_string(),
    };

    let rgb_client = RGBLibClient::new(rgb_lib_config.clone()).await.unwrap();

    println!(
        "RGB Wallet created in directory `{}` with mnemonic: `{}`",
        rgb_lib_config.data_dir, rgb_lib_config.mnemonic
    );

    server
        .nest_router(
            "/rgb",
            RGBHandler::routes(Arc::new(rgb_client) as DynRGBClient),
        )
        .await
        .unwrap();

    server
        .nest_router("/.well-known", LightningHandler::well_known_routes())
        .await
        .unwrap();

    let breez_config = BreezClientConfig {
        api_key: "ea3d7d992a5e886ef36ca779b6e81afc80380624bd6f8341e6c75ce6de60a1f4".to_string(),
        invite_code: "89NL-AKGQ".to_string(),
        working_dir: "storage/breez".to_string(),
        seed: "frost only system trade august ritual pyramid bracket range appear camp earth"
            .to_string(),
    };
    let lightning_client = BreezClient::new(breez_config.clone()).await.unwrap();

    println!(
        "Breez client created in directory `{}` with mnemonic: `{}`",
        breez_config.working_dir, breez_config.seed
    );
    server
        .nest_router(
            "/lightning",
            LightningHandler::routes(Arc::new(lightning_client) as DynLightningClient),
        )
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
