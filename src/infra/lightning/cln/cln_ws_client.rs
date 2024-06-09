use anyhow::{anyhow, Result};
use axum::http::Uri;
use futures_util::StreamExt;
use std::time::Duration;
use tokio::time::sleep;
use tokio_tungstenite::{
    connect_async,
    tungstenite::{error::Error, ClientRequestBuilder, Message},
};
use tracing::{debug, error, warn};

use crate::application::errors::LightningError;

pub struct ClnWsClient {
    client: ClientRequestBuilder,
    retry_delay: Duration,
}

impl ClnWsClient {
    pub async fn new(
        endpoint: String,
        rune: String,
        retry_delay: Duration,
    ) -> Result<Self, LightningError> {
        let uri = endpoint
            .parse::<Uri>()
            .map_err(|e| LightningError::ParseConfig(e.to_string()))?;

        let client = ClientRequestBuilder::new(uri).with_header("rune", rune);

        Ok(Self {
            client,
            retry_delay,
        })
    }

    pub async fn listen(&self) -> Result<()> {
        loop {
            match connect_async(self.client.clone()).await {
                Ok((mut socket, response)) => {
                    debug!(?response, "Connected to WebSocket server");

                    while let Some(msg) = socket.next().await {
                        match msg {
                            Ok(Message::Text(text)) => {
                                println!("Received: {}", text);
                            }
                            Ok(Message::Binary(bin)) => {
                                println!("Received binary: {:?}", bin);
                            }
                            Ok(Message::Close(frame)) => {
                                error!(?frame, "Received close message retrying in 5 seconds");
                                sleep(self.retry_delay).await;
                                break;
                            }
                            Ok(msg) => {
                                warn!(?msg, "Received other message type");
                            }
                            Err(err) => {
                                error!(?err, "Failed to read message");
                                break;
                            }
                        }
                    }
                }
                Err(err) => match err {
                    Error::Tls(e) => {
                        return Err(anyhow!(e.to_string()));
                    }
                    Error::Io(e) => {
                        return Err(anyhow!(e.to_string()));
                    }
                    Error::Protocol(e) => {
                        return Err(anyhow!(e.to_string()));
                    }
                    _ => {
                        error!(
                            ?err,
                            "Failed to connect to WebSocket server, retrying in 5 seconds"
                        );
                        sleep(self.retry_delay).await;
                    }
                },
            }
        }
    }
}
