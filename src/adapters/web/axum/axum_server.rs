use std::{
    net::{AddrParseError, SocketAddr},
    pin::Pin,
    sync::Arc,
};

use async_trait::async_trait;
use axum::{Router, Server};
use futures_util::future::poll_fn;
use hyper::{
    server::{
        accept::Accept,
        conn::{AddrIncoming, Http},
    },
    service::Service,
};
use tokio::{net::TcpListener, sync::Mutex};
use tokio_rustls::{rustls::ServerConfig, TlsAcceptor};

use crate::{adapters::web::WebServer, application::errors::WebServerError};

use super::AxumServerConfig;

pub struct AxumServer {
    addr: SocketAddr,
    router: Arc<Mutex<Router>>,
    tls_config: Option<Arc<ServerConfig>>,
}

impl AxumServer {
    pub fn new(config: AxumServerConfig) -> Result<Self, WebServerError> {
        let router = Arc::new(Mutex::new(Router::new()));
        let addr: SocketAddr = config
            .addr
            .parse()
            .map_err(|e: AddrParseError| WebServerError::Parse(e.to_string()))?;

        Ok(Self {
            router,
            addr,
            tls_config: config.tls_config(),
        })
    }
}

#[async_trait]
impl WebServer for AxumServer {
    async fn start(&self) -> Result<(), WebServerError> {
        let router = {
            let lock = self.router.lock().await;
            lock.clone()
        };

        if let Some(tls_config) = self.tls_config.clone() {
            let acceptor = TlsAcceptor::from(tls_config);

            let listener = TcpListener::bind(self.addr).await.unwrap();
            let mut listener = AddrIncoming::from_listener(listener).unwrap();

            let protocol = Arc::new(Http::new());

            let mut app = router.into_make_service_with_connect_info::<SocketAddr>();

            loop {
                let stream = poll_fn(|cx| Pin::new(&mut listener).poll_accept(cx))
                    .await
                    .unwrap()
                    .unwrap();

                let acceptor = acceptor.clone();
                let protocol = protocol.clone();

                let svc = app.call(&stream);

                tokio::spawn(async move {
                    if let Ok(stream) = acceptor.accept(stream).await {
                        let _ = protocol.serve_connection(stream, svc.await.unwrap()).await;
                    }
                });
            }
        } else {
            Server::bind(&self.addr)
                .serve(router.into_make_service())
                .await
                .map_err(|e| WebServerError::Listener(e.to_string()))?;

            Ok(())
        }
    }

    async fn nest_router(&mut self, path: &str, nested_router: Router) -> &mut Self {
        let old_router = {
            let lock = self.router.lock().await;
            (*lock).clone()
        };

        let new_router = old_router.clone().nest(path, nested_router);
        self.router = Arc::new(Mutex::new(new_router));

        self
    }
}
