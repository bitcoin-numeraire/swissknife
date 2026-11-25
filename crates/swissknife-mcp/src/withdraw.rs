use anyhow::{anyhow, ensure, Context};
use lnurl::{lnurl::LnUrl, withdraw::WithdrawalResponse, Tag};
use reqwest::{Client, Url};
use serde_json::{json, Value};

const MAX_RESPONSE_BYTES: usize = 32 * 1024;

#[derive(Clone)]
pub(super) struct WithdrawalClient {
    http: Client,
    origins: Vec<Url>,
}

pub(super) struct Offer {
    issuer: Url,
    callback: Url,
    terms: WithdrawalResponse,
}

impl Offer {
    pub(super) fn terms(&self) -> Value {
        json!({
            "issuer": self.issuer.origin().ascii_serialization(),
            "callback_origin": self.callback.origin().ascii_serialization(),
            "description": self.terms.default_description,
            "min_msat": self.terms.min_withdrawable.unwrap_or(1).to_string(),
            "max_msat": self.terms.max_withdrawable.to_string(),
        })
    }

    pub(super) fn check_amount(&self, amount: u64) -> anyhow::Result<()> {
        ensure!(
            amount > 0 && amount >= self.terms.min_withdrawable.unwrap_or(1) && amount <= self.terms.max_withdrawable,
            "Choose a positive amount within the voucher range"
        );
        Ok(())
    }
}

impl WithdrawalClient {
    pub(super) fn new(origins: &[String]) -> anyhow::Result<Self> {
        let mut trusted = Vec::new();
        for value in origins {
            let origin = Url::parse(value).context("Invalid withdrawal service origin")?;
            ensure!(
                super::secure_service_url(&origin)
                    && origin.path() == "/"
                    && origin.query().is_none()
                    && origin.fragment().is_none(),
                "Configure an HTTPS origin, or HTTP on a loopback IP"
            );
            trusted.push(origin);
        }
        // Withdrawal services receive invoices. Only the separate WalletClient holds a wallet key.
        let http = super::external_client()?;
        Ok(Self { http, origins: trusted })
    }

    pub(super) fn enabled(&self) -> bool {
        !self.origins.is_empty()
    }

    fn service_url(&self, raw: &str) -> anyhow::Result<Url> {
        let url = Url::parse(raw).map_err(|_| anyhow!("Invalid withdrawal service URL"))?;
        ensure!(
            url.username().is_empty()
                && url.password().is_none()
                && url.fragment().is_none()
                && self.origins.iter().any(|trusted| trusted.origin() == url.origin()),
            "Withdrawal service origin is not configured for this bridge"
        );
        Ok(url)
    }

    async fn read_json(&self, url: Url) -> anyhow::Result<Value> {
        // Transport errors may contain the voucher URL, k1 or invoice; expose only the protocol outcome.
        let mut response = self
            .http
            .get(url)
            .send()
            .await
            .map_err(|_| anyhow!("Withdrawal service did not respond"))?;
        ensure!(
            response.status().is_success(),
            "Withdrawal service returned an unsuccessful HTTP status"
        );
        let mut bytes = Vec::new();
        while let Some(chunk) = response
            .chunk()
            .await
            .map_err(|_| anyhow!("Incomplete withdrawal service response"))?
        {
            ensure!(
                bytes.len() + chunk.len() <= MAX_RESPONSE_BYTES,
                "Withdrawal service response is too large"
            );
            bytes.extend_from_slice(&chunk);
        }
        serde_json::from_slice(&bytes).map_err(|_| anyhow!("Invalid withdrawal service response"))
    }

    pub(super) async fn discover(&self, encoded: &str) -> anyhow::Result<Offer> {
        let encoded = encoded.trim();
        let encoded = encoded
            .split_once(':')
            .filter(|(s, _)| s.eq_ignore_ascii_case("lightning"))
            .map_or(encoded, |(_, s)| s);
        ensure!(
            encoded.len() <= 8192 && encoded.get(..6).is_some_and(|p| p.eq_ignore_ascii_case("lnurl1")),
            "Expected an encoded LNURL voucher"
        );
        let link = encoded
            .parse::<LnUrl>()
            .map_err(|_| anyhow!("Invalid encoded LNURL voucher"))?;
        let issuer = self.service_url(&link.url)?;
        let data = self.read_json(issuer.clone()).await?;
        ensure!(data["status"] != "ERROR", "Withdrawal service declined the voucher");
        let terms: WithdrawalResponse =
            serde_json::from_value(data).map_err(|_| anyhow!("Invalid LNURL withdrawal terms"))?;
        ensure!(
            terms.tag == Tag::WithdrawRequest && !terms.k1.is_empty(),
            "Expected an LNURL withdrawal challenge"
        );
        ensure!(
            terms.min_withdrawable.unwrap_or(1) <= terms.max_withdrawable,
            "Invalid withdrawal range"
        );
        let callback = self.service_url(&terms.callback)?;
        Ok(Offer {
            issuer,
            callback,
            terms,
        })
    }

    pub(super) async fn submit(&self, offer: Offer, invoice: &str) -> &'static str {
        let mut callback = offer.callback;
        let params: Vec<_> = callback
            .query_pairs()
            .into_owned()
            .filter(|(key, _)| key != "pr" && key != "k1")
            .collect();
        callback
            .query_pairs_mut()
            .clear()
            .extend_pairs(params)
            .append_pair("k1", &offer.terms.k1)
            .append_pair("pr", invoice);
        match self.read_json(callback).await {
            Ok(data) if data["status"] == "OK" => "accepted",
            Ok(data) if data["status"] == "ERROR" => "rejected",
            _ => "unknown",
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn configures_exact_service_origins_and_accepts_loopback_development_services() {
        let services = vec!["https://voucher.example".to_owned(), "http://127.0.0.1:3001".to_owned()];
        let client = WithdrawalClient::new(&services).unwrap();
        assert!(client.service_url("https://voucher.example/redeem?ref=1").is_ok());
        assert!(client.service_url("http://127.0.0.1:3001/offer").is_ok());
        for other_origin in ["https://elsewhere.example", "https://voucher.example:8443/offer"] {
            assert!(client.service_url(other_origin).is_err());
        }
        for invalid in [
            "http://voucher.example",
            "https://voucher.example/path",
            "https://voucher.example/?order=1",
        ] {
            assert!(WithdrawalClient::new(&[invalid.to_string()]).is_err());
        }
    }
}
