use adapters::web::{
    axum::{AxumServer, AxumServerConfig},
    web::WebServer,
};
use rgb::api::http::rgb_controller::RGBController;

mod adapters;
mod rgb;

#[tokio::main]
async fn main() {
    let addr = "0.0.0.0:3000";

    // Create app
    let server = AxumServer::new(AxumServerConfig {
        addr: addr.to_string(),
    })
    .unwrap();

    server
        .nest_router("/rgb", RGBController::new().routes())
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
