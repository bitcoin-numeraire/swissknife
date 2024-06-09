use std::sync::Arc;

use anyhow::Result;
use axum::Router;

use tokio::{
    net::TcpListener,
    signal::{
        ctrl_c,
        unix::{signal, SignalKind},
    },
    task::{spawn, JoinHandle},
};
use tower_http::{cors::CorsLayer, trace::TraceLayer};
use tracing::{debug, error, info};

use crate::{
    application::errors::WebServerError,
    domains::{
        invoices::api::InvoiceHandler,
        lightning::api::{LnAddressHandler, LnNodeHandler, LnURLpHandler},
        payments::api::PaymentHandler,
        users::api::WalletHandler,
    },
    infra::app::AppState,
};

pub async fn start(state: Arc<AppState>, addr: &str) -> Result<()> {
    let router = Router::new()
        .nest("/.well-known/lnurlp", LnURLpHandler::well_known_route())
        .nest("/api/lnurlp", LnURLpHandler::callback_route())
        .nest("/api/lightning/addresses", LnAddressHandler::routes())
        .nest("/api/lightning/node", LnNodeHandler::routes())
        .nest("/api/invoices", InvoiceHandler::routes())
        .nest("/api/payments", PaymentHandler::routes())
        .nest("/api/wallet", WalletHandler::routes())
        .layer(TraceLayer::new_for_http())
        .layer(state.timeout_layer)
        .layer(CorsLayer::permissive())
        .with_state(state.clone());

    let listener = TcpListener::bind(addr)
        .await
        .map_err(|e| WebServerError::Listener(e.to_string()))?;

    // Start Lightning node listener
    let listener_handle: JoinHandle<Result<()>> = spawn(async move {
        state.ln_client.listen_events().await?;
        Ok(())
    });

    // Start Web server
    let server_handle: JoinHandle<Result<()>> = spawn(async move {
        axum::serve(listener, router)
            .with_graceful_shutdown(shutdown_signal())
            .await
            .map_err(|e| WebServerError::Serve(e.to_string()))?;

        Ok(())
    });
    info!(addr, "Web server listening on address");

    tokio::select! {
        res = server_handle => {
            match res {
                Ok(Ok(())) => {
                },
                Ok(Err(err)) => {
                    return Err(err.into());
                },
                Err(err) => {
                    return Err(err.into());
                }
            }
        },
        res = listener_handle => {
            match res {
                Ok(Ok(())) => {
                },
                Ok(Err(err)) => {
                    return Err(err.into());
                },
                Err(err) => {
                    return Err(err.into());
                }
            }
        },
        _ = shutdown_signal() => {
            info!("Received shutdown signal. Shutting down gracefully");
        },
    }

    Ok(())
}

async fn shutdown_signal() {
    let ctrl_c = async {
        if let Err(err) = ctrl_c().await {
            error!(%err, "Failed to install Ctrl+C handler");
        }

        debug!("Received Ctrl+C");
    };

    #[cfg(unix)]
    let terminate = async {
        match signal(SignalKind::terminate()) {
            Ok(mut stream) => {
                stream.recv().await;
                info!("Received SIGTERM. Shutting down gracefully");
            }
            Err(err) => error!(%err, "Failed to install SIGTERM handler"),
        }
    };

    #[cfg(not(unix))]
    let terminate = std::future::pending::<()>();

    tokio::select! {
        _ = ctrl_c => {},
        _ = terminate => {},
    }
}
