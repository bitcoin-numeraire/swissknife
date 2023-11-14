use adapters::web::{
    axum::{AxumServer, AxumServerConfig},
    web::WebServer,
};

mod adapters;

#[tokio::main]
async fn main() {
    let addr = "0.0.0.0:3000";

    let server = AxumServer::new(AxumServerConfig {
        addr: addr.to_string(),
    })
    .unwrap();
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
