use std::time::Duration;

use reqwest::{header::CACHE_CONTROL, Client, Url};
use serde::Deserialize;
use swissknife_l402::GateError;

const MAX_QUOTE_BYTES: usize = 1024;

pub(super) struct Price {
    maximum_msat: u64,
    callback: Option<(Client, Url)>,
}

impl Price {
    pub(super) fn new(maximum_msat: u64, url: Option<Url>, client: &Client) -> Self {
        Self {
            maximum_msat,
            callback: url.map(|url| (client.clone(), url)),
        }
    }

    pub(super) async fn quote(&self) -> Result<u64, GateError> {
        let Some((client, url)) = &self.callback else {
            return Ok(self.maximum_msat);
        };
        let mut response = client
            .get(url.clone())
            .header(CACHE_CONTROL, "no-cache")
            .timeout(Duration::from_secs(3))
            .send()
            .await
            .map_err(|_| GateError::Unavailable)?;
        if !response.status().is_success() || response.content_length().is_some_and(|n| n > MAX_QUOTE_BYTES as u64) {
            return Err(GateError::Unavailable);
        }

        let mut body = Vec::new();
        while let Some(chunk) = response.chunk().await.map_err(|_| GateError::Unavailable)? {
            if chunk.len() > MAX_QUOTE_BYTES - body.len() {
                return Err(GateError::Unavailable);
            }
            body.extend_from_slice(&chunk);
        }
        #[derive(Deserialize)]
        struct Quote {
            price_msat: u64,
        }

        let quote: Quote = serde_json::from_slice(&body).map_err(|_| GateError::Unavailable)?;
        if !(1..=self.maximum_msat).contains(&quote.price_msat) {
            return Err(GateError::Unavailable);
        }
        Ok(quote.price_msat)
    }
}
