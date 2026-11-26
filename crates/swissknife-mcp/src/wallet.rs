use anyhow::{bail, ensure, Context};
use reqwest::{
    header::{HeaderMap, HeaderValue, ACCEPT},
    Client, Method, Url,
};
use rmcp::schemars;
use serde::Deserialize;
use serde_json::{json, Value};
use std::{net::IpAddr, time::Duration};
use swissknife_types::{Ledger, PaymentFilter, PaymentStatus};
use uuid::Uuid;

#[derive(Default, Deserialize, schemars::JsonSchema)]
#[serde(deny_unknown_fields)]
pub(super) struct ListPayments {
    /// Maximum matching payments to return, from 1 to 100. Defaults to 20.
    #[schemars(range(min = 1, max = 100))]
    limit: Option<u8>,
    /// Skip this many matching payments.
    offset: Option<u32>,
    /// Only payments with this lifecycle status.
    status: Option<PaymentStatus>,
    /// Only payments on this ledger.
    ledger: Option<Ledger>,
}

#[derive(Clone)]
pub struct WalletClient {
    http: Client,
    base: Url,
}

impl WalletClient {
    pub fn new(server: &str, wallet: Uuid, secret: &str) -> anyhow::Result<Self> {
        let mut base = Url::parse(server).context("Invalid SwissKnife origin")?;
        let host = base.host_str().context("SwissKnife origin has no hostname")?;
        let local = host == "localhost"
            || host
                .trim_matches(['[', ']'])
                .parse::<IpAddr>()
                .is_ok_and(|ip| ip.is_loopback());
        ensure!(
            base.scheme() == "https" || (base.scheme() == "http" && local),
            "Use HTTPS or loopback HTTP for the SwissKnife origin"
        );
        ensure!(
            base.username().is_empty()
                && base.password().is_none()
                && base.path() == "/"
                && base.query().is_none()
                && base.fragment().is_none(),
            "Supply only the SwissKnife origin"
        );
        let encoded = secret
            .strip_prefix("swka_")
            .context("Expected a delegated wallet credential")?;
        ensure!(
            encoded.len() == 64 && encoded.bytes().all(|b| b.is_ascii_hexdigit()),
            "Invalid delegated wallet credential"
        );
        base.set_path(&format!("/v1/me/wallets/{wallet}/"));
        let mut headers = HeaderMap::new();
        let mut auth = HeaderValue::from_str(secret)?;
        auth.set_sensitive(true);
        headers.insert("wallet-key", auth);
        headers.insert(ACCEPT, HeaderValue::from_static("application/json"));
        let http = Client::builder()
            .default_headers(headers)
            .redirect(reqwest::redirect::Policy::none())
            .timeout(Duration::from_secs(30))
            .build()?;
        Ok(Self { http, base })
    }

    pub async fn get(&self, path: &str) -> anyhow::Result<Value> {
        self.request(Method::GET, path, None).await
    }

    pub async fn post(&self, path: &str, body: Value) -> anyhow::Result<Value> {
        self.request(Method::POST, path, Some(body)).await
    }

    async fn request(&self, method: Method, path: &str, body: Option<Value>) -> anyhow::Result<Value> {
        // Only this crate supplies relative paths. IDs are parsed as UUIDs before interpolation.
        let url = if path.is_empty() {
            let mut url = self.base.clone();
            url.path_segments_mut().expect("configured HTTP origin").pop_if_empty();
            url
        } else {
            self.base.join(path)?
        };
        let mut request = self.http.request(method, url);
        if let Some(body) = body {
            request = request.json(&body);
        }
        let mut response = request
            .send()
            .await
            .context("SwissKnife request did not return a response")?;
        let status = response.status();
        let mut bytes = Vec::new();
        while let Some(chunk) = response.chunk().await? {
            ensure!(
                bytes.len() + chunk.len() <= 2 * 1024 * 1024,
                "SwissKnife response exceeded 2 MiB"
            );
            bytes.extend_from_slice(&chunk);
        }
        let mut value = serde_json::from_slice::<Value>(&bytes).unwrap_or(Value::Null);
        if !status.is_success() {
            let reason = value["reason"].as_str().unwrap_or("Request rejected");
            let message = format!("SwissKnife {}: {}", status.as_u16(), reason);
            if let Err(error) = response.error_for_status_ref() {
                return Err(anyhow::Error::new(error.without_url()).context(message));
            }
            bail!(message);
        }
        ensure!(!value.is_null(), "SwissKnife returned an empty or invalid response");
        exact_amounts(&mut value);
        Ok(value)
    }

    pub(super) async fn recent(&self, args: ListPayments) -> anyhow::Result<Value> {
        let limit = args.limit.unwrap_or(20);
        ensure!((1..=100).contains(&limit), "Choose a limit from 1 to 100");
        let query = serde_urlencoded::to_string(PaymentFilter {
            limit: Some(u64::from(limit)),
            offset: Some(u64::from(args.offset.unwrap_or(0))),
            status: args.status,
            ledger: args.ledger,
            ..Default::default()
        })?;
        let payments = self.get(&format!("payments?{query}")).await?;
        Ok(json!({"payments":payments}))
    }
}

/// A JSON number can lose precision in JavaScript clients. Expose all monetary
/// integers as decimal strings, including signed balances and nested results.
fn exact_amounts(value: &mut Value) {
    match value {
        Value::Object(fields) => {
            for (name, field) in fields {
                if name.ends_with("_msat") && field.is_number() {
                    *field = Value::String(field.to_string());
                } else {
                    exact_amounts(field);
                }
            }
        }
        Value::Array(values) => values.iter_mut().for_each(exact_amounts),
        _ => {}
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn preserves_large_positive_and_negative_amounts_for_javascript_clients() {
        let mut payment = json!({"id": 3, "fees_paid_msat": 9007199254740993u64,
            "wallets": [{"available_msat": -9007199254740993i64, "amount_msat": null}]});
        exact_amounts(&mut payment);
        assert_eq!(
            payment,
            json!({"id": 3, "fees_paid_msat": "9007199254740993",
            "wallets": [{"available_msat": "-9007199254740993", "amount_msat": null}]})
        );
    }
}
