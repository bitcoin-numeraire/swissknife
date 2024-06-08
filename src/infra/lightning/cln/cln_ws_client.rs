use crate::application::errors::LightningError;
use http::HeaderValue;
use std::net::TcpStream;
use std::time::Duration;
use tokio::task;
use tokio::time;
use tracing::warn;
use tracing::{debug, error, info};
use tungstenite::{client::IntoClientRequest, connect, stream::MaybeTlsStream, Message, WebSocket};

use super::ClnRestClientConfig;

pub struct ClnWsClient {
    config: ClnRestClientConfig,
    socket: Option<WebSocket<MaybeTlsStream<TcpStream>>>,
}

impl ClnWsClient {
    pub fn new(config: ClnRestClientConfig) -> Result<Self, LightningError> {
        let socket = Self::connect(&config)?;
        Ok(Self {
            config,
            socket: Some(socket),
        })
    }

    fn connect(
        config: &ClnRestClientConfig,
    ) -> Result<WebSocket<MaybeTlsStream<TcpStream>>, LightningError> {
        let mut req = config
            .ws_endpoint
            .clone()
            .into_client_request()
            .map_err(|e| LightningError::ParseConfig(e.to_string()))?;

        req.headers_mut().insert(
            "rune",
            HeaderValue::from_str(&config.rune)
                .map_err(|e| LightningError::ParseConfig(e.to_string()))?,
        );

        let (socket, response) =
            connect(req).map_err(|e| LightningError::ConnectWebSocket(e.to_string()))?;

        debug!(?response, "Connected to WebSocket server");

        Ok(socket)
    }

    pub fn listen(mut self) {
        task::spawn(async move {
            loop {
                if self.socket.is_none() {
                    match self.reconnect().await {
                        Ok(_) => info!("Reconnected to WebSocket"),
                        Err(e) => {
                            error!(?e, "Reconnection failed");
                            time::sleep(Duration::from_secs(5)).await;
                            continue;
                        }
                    }
                }

                if let Some(socket) = &mut self.socket {
                    match socket.read() {
                        Ok(msg) => match msg {
                            Message::Text(text) => {
                                println!("Received: {}", text);
                            }
                            Message::Binary(bin) => {
                                println!("Received binary: {:?}", bin);
                            }
                            Message::Close(frame) => {
                                error!(?frame, "Received close message");
                                self.socket = None;
                            }
                            _ => {
                                warn!(?msg, "Received other message type");
                            }
                        },
                        Err(err) => {
                            error!(?err, "Failed to read message");
                            self.socket = None;
                        }
                    }
                }
            }
        });
    }

    async fn reconnect(&mut self) -> Result<(), LightningError> {
        self.socket = None;
        let mut attempts = 0;
        let max_attempts = 5;
        let delay = Duration::from_secs(5);

        while attempts < max_attempts {
            attempts += 1;
            match Self::connect(&self.config) {
                Ok(new_socket) => {
                    self.socket = Some(new_socket);
                    return Ok(());
                }
                Err(err) => {
                    error!(
                        ?err,
                        "Reconnection attempt {}/{} failed. Retrying in {} seconds...",
                        attempts,
                        max_attempts,
                        delay.as_secs()
                    );
                    time::sleep(delay).await;
                }
            }
        }

        error!(
            "Failed to reconnect after {} attempts. Giving up.",
            max_attempts
        );
        Err(LightningError::ConnectWebSocket(
            "Failed to reconnect".to_string(),
        ))
    }
}
